#import "calendar_apple.h"
#import <Foundation/Foundation.h>
#import <EventKit/EventKit.h>

static char *create_c_string(NSString *str) {
    if (!str) return NULL;
    const char *utf8 = [str UTF8String];
    return utf8 ? strdup(utf8) : NULL;
}

void calendar_apple_free_string(char *ptr) {
    if (ptr) {
        free(ptr);
    }
}

int calendar_apple_check_permission(char **out_status, char **out_error) {
    (void)out_error;
    @autoreleasepool {
        EKAuthorizationStatus status = [EKEventStore authorizationStatusForEntityType:EKEntityTypeEvent];
        NSString *statusStr = @"unknown";

#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wdeprecated-declarations"
        switch (status) {
            case EKAuthorizationStatusNotDetermined:
                statusStr = @"not_determined";
                break;
            case EKAuthorizationStatusRestricted:
                statusStr = @"restricted";
                break;
            case EKAuthorizationStatusDenied:
                statusStr = @"denied";
                break;
            case EKAuthorizationStatusAuthorized:
                statusStr = @"authorized";
                break;
            default:
                // EKAuthorizationStatusWriteOnly (value 4) introduced in iOS 17.0 / macOS 14.0
                if ((NSInteger)status == 4) {
                    statusStr = @"write_only";
                } else {
                    statusStr = @"unknown";
                }
                break;
        }
#pragma clang diagnostic pop
        if (out_status) {
            *out_status = create_c_string(statusStr);
        }
        return 0;
    }
}

int calendar_apple_request_permission(int *out_granted, char **out_error) {
    @autoreleasepool {
        EKEventStore *store = [[EKEventStore alloc] init];
        __block BOOL grantedResult = NO;
        __block NSError *errorResult = nil;
        dispatch_semaphore_t sem = dispatch_semaphore_create(0);

        if (@available(iOS 17.0, macOS 14.0, *)) {
            // iOS 17+ supports requesting write-only access for creating events
            [store requestWriteOnlyAccessToEventsWithCompletion:^(BOOL granted, NSError * _Nullable error) {
                grantedResult = granted;
                errorResult = error;
                dispatch_semaphore_signal(sem);
            }];
        } else {
            #pragma clang diagnostic push
            #pragma clang diagnostic ignored "-Wdeprecated-declarations"
            [store requestAccessToEntityType:EKEntityTypeEvent completion:^(BOOL granted, NSError * _Nullable error) {
                grantedResult = granted;
                errorResult = error;
                dispatch_semaphore_signal(sem);
            }];
            #pragma clang diagnostic pop
        }

        // Wait up to 60 seconds for user prompt interaction
        long waitResult = dispatch_semaphore_wait(sem, dispatch_time(DISPATCH_TIME_NOW, 60 * NSEC_PER_SEC));
        if (waitResult != 0) {
            if (out_error) {
                *out_error = create_c_string(@"Calendar permission request timed out.");
            }
            return -1;
        }

        if (errorResult) {
            if (out_error) {
                *out_error = create_c_string(errorResult.localizedDescription);
            }
            return -1;
        }

        if (out_granted) {
            *out_granted = grantedResult ? 1 : 0;
        }
        return 0;
    }
}

int calendar_apple_create_event(
    const char *title,
    double start_epoch,
    double end_epoch,
    int is_all_day,
    const char *location,
    const char *notes,
    const char *url,
    char **out_event_id,
    char **out_error
) {
    if (!title || strlen(title) == 0) {
        if (out_error) {
            *out_error = create_c_string(@"Event title is required.");
        }
        return -1;
    }

    @autoreleasepool {
        EKEventStore *store = [[EKEventStore alloc] init];

        // Check current authorization status
        EKAuthorizationStatus status = [EKEventStore authorizationStatusForEntityType:EKEntityTypeEvent];
        if (status == EKAuthorizationStatusNotDetermined) {
            int granted = 0;
            char *reqErr = NULL;
            int reqRes = calendar_apple_request_permission(&granted, &reqErr);
            if (reqRes != 0 || !granted) {
                if (out_error) {
                    *out_error = reqErr ? reqErr : create_c_string(@"Calendar access was not granted by the user.");
                } else if (reqErr) {
                    calendar_apple_free_string(reqErr);
                }
                return -1;
            }
            if (reqErr) {
                calendar_apple_free_string(reqErr);
            }
            // Re-initialize store after permission grant
            store = [[EKEventStore alloc] init];
        } else if (status == EKAuthorizationStatusDenied || status == EKAuthorizationStatusRestricted) {
            if (out_error) {
                *out_error = create_c_string(@"Calendar access denied. Please enable Calendar permissions in iOS Settings.");
            }
            return -1;
        }

        // Retrieve default calendar or find first writable calendar
        EKCalendar *targetCalendar = [store defaultCalendarForNewEvents];
        if (!targetCalendar || !targetCalendar.allowsContentModifications) {
            NSArray<EKCalendar *> *calendars = [store calendarsForEntityType:EKEntityTypeEvent];
            for (EKCalendar *cal in calendars) {
                if (cal.allowsContentModifications) {
                    targetCalendar = cal;
                    break;
                }
            }
        }

        if (!targetCalendar) {
            if (out_error) {
                *out_error = create_c_string(@"No writable calendar found on device.");
            }
            return -1;
        }

        EKEvent *event = [EKEvent eventWithEventStore:store];
        event.title = [NSString stringWithUTF8String:title];
        event.calendar = targetCalendar;
        event.allDay = (is_all_day != 0);

        NSDate *startDate = [NSDate dateWithTimeIntervalSince1970:start_epoch];
        event.startDate = startDate;

        if (end_epoch > start_epoch) {
            event.endDate = [NSDate dateWithTimeIntervalSince1970:end_epoch];
        } else if (is_all_day) {
            event.endDate = startDate;
        } else {
            // Default to 1 hour duration if end time is missing or before start time
            event.endDate = [startDate dateByAddingTimeInterval:3600];
        }

        if (location && strlen(location) > 0) {
            event.location = [NSString stringWithUTF8String:location];
        }

        if (notes && strlen(notes) > 0) {
            event.notes = [NSString stringWithUTF8String:notes];
        }

        if (url && strlen(url) > 0) {
            NSString *urlStr = [NSString stringWithUTF8String:url];
            NSURL *nsUrl = [NSURL URLWithString:urlStr];
            if (nsUrl) {
                event.URL = nsUrl;
            }
        }

        NSError *saveError = nil;
        BOOL success = [store saveEvent:event span:EKSpanThisEvent commit:YES error:&saveError];
        if (!success || saveError) {
            if (out_error) {
                *out_error = create_c_string(saveError ? saveError.localizedDescription : @"Failed to save event to calendar.");
            }
            return -1;
        }

        NSString *eventId = event.eventIdentifier;
        if (!eventId || eventId.length == 0) {
            eventId = @"saved";
        }

        if (out_event_id) {
            *out_event_id = create_c_string(eventId);
        }

        return 0;
    }
}
