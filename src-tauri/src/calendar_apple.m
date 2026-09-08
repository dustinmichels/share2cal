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

static EKRecurrenceRule *parse_rrule_string(NSString *rruleStr, NSTimeZone *eventTimeZone, NSString **outError) {
    if (!rruleStr || rruleStr.length == 0) return nil;

    NSString *rule = [rruleStr stringByTrimmingCharactersInSet:[NSCharacterSet whitespaceAndNewlineCharacterSet]];
    if ([rule hasPrefix:@"RRULE:"]) {
        rule = [rule substringFromIndex:6];
    }

    NSArray<NSString *> *parts = [rule componentsSeparatedByString:@";"];
    EKRecurrenceFrequency frequency = EKRecurrenceFrequencyWeekly;
    BOOL hasFreq = NO;
    NSInteger interval = 1;
    NSMutableArray<EKRecurrenceDayOfWeek *> *daysOfTheWeek = [NSMutableArray array];
    EKRecurrenceEnd *recurrenceEnd = nil;

    for (NSString *part in parts) {
        NSString *trimmedPart = [part stringByTrimmingCharactersInSet:[NSCharacterSet whitespaceAndNewlineCharacterSet]];
        if (trimmedPart.length == 0) continue;

        NSRange eqRange = [trimmedPart rangeOfString:@"="];
        if (eqRange.location == NSNotFound) {
            if (outError) *outError = [NSString stringWithFormat:@"Malformed recurrence property (missing '='): %@", trimmedPart];
            return nil;
        }

        NSString *key = [[trimmedPart substringToIndex:eqRange.location] stringByTrimmingCharactersInSet:[NSCharacterSet whitespaceAndNewlineCharacterSet]].uppercaseString;
        NSString *val = [[trimmedPart substringFromIndex:eqRange.location + 1] stringByTrimmingCharactersInSet:[NSCharacterSet whitespaceAndNewlineCharacterSet]];

        if (key.length == 0 || val.length == 0) {
            if (outError) *outError = [NSString stringWithFormat:@"Empty key or value in recurrence property: %@", trimmedPart];
            return nil;
        }

        if ([key isEqualToString:@"FREQ"]) {
            NSString *freqVal = [val uppercaseString];
            if ([freqVal isEqualToString:@"DAILY"]) {
                frequency = EKRecurrenceFrequencyDaily;
                hasFreq = YES;
            } else if ([freqVal isEqualToString:@"WEEKLY"]) {
                frequency = EKRecurrenceFrequencyWeekly;
                hasFreq = YES;
            } else if ([freqVal isEqualToString:@"MONTHLY"]) {
                frequency = EKRecurrenceFrequencyMonthly;
                hasFreq = YES;
            } else if ([freqVal isEqualToString:@"YEARLY"]) {
                frequency = EKRecurrenceFrequencyYearly;
                hasFreq = YES;
            } else {
                if (outError) *outError = [NSString stringWithFormat:@"Invalid FREQ value: %@", val];
                return nil;
            }
        } else if ([key isEqualToString:@"INTERVAL"]) {
            NSInteger intVal = [val integerValue];
            if (intVal <= 0) {
                if (outError) *outError = [NSString stringWithFormat:@"INTERVAL must be a positive integer: %@", val];
                return nil;
            }
            interval = intVal;
        } else if ([key isEqualToString:@"BYDAY"]) {
            NSArray<NSString *> *dayTokens = [[val uppercaseString] componentsSeparatedByString:@","];
            for (NSString *dt in dayTokens) {
                NSString *dayCode = [dt stringByTrimmingCharactersInSet:[NSCharacterSet whitespaceAndNewlineCharacterSet]];
                if ([dayCode isEqualToString:@"MO"]) {
                    [daysOfTheWeek addObject:[EKRecurrenceDayOfWeek dayOfWeek:EKWeekdayMonday]];
                } else if ([dayCode isEqualToString:@"TU"]) {
                    [daysOfTheWeek addObject:[EKRecurrenceDayOfWeek dayOfWeek:EKWeekdayTuesday]];
                } else if ([dayCode isEqualToString:@"WE"]) {
                    [daysOfTheWeek addObject:[EKRecurrenceDayOfWeek dayOfWeek:EKWeekdayWednesday]];
                } else if ([dayCode isEqualToString:@"TH"]) {
                    [daysOfTheWeek addObject:[EKRecurrenceDayOfWeek dayOfWeek:EKWeekdayThursday]];
                } else if ([dayCode isEqualToString:@"FR"]) {
                    [daysOfTheWeek addObject:[EKRecurrenceDayOfWeek dayOfWeek:EKWeekdayFriday]];
                } else if ([dayCode isEqualToString:@"SA"]) {
                    [daysOfTheWeek addObject:[EKRecurrenceDayOfWeek dayOfWeek:EKWeekdaySaturday]];
                } else if ([dayCode isEqualToString:@"SU"]) {
                    [daysOfTheWeek addObject:[EKRecurrenceDayOfWeek dayOfWeek:EKWeekdaySunday]];
                } else {
                    if (outError) *outError = [NSString stringWithFormat:@"Invalid BYDAY token: %@", dt];
                    return nil;
                }
            }
        } else if ([key isEqualToString:@"UNTIL"]) {
            NSDateFormatter *df = [[NSDateFormatter alloc] init];
            df.locale = [NSLocale localeWithLocaleIdentifier:@"en_US_POSIX"];

            NSDate *untilDate = nil;
            if ([val hasSuffix:@"Z"] || [val hasSuffix:@"z"]) {
                df.timeZone = [NSTimeZone timeZoneWithAbbreviation:@"UTC"];
                df.dateFormat = @"yyyyMMdd'T'HHmmss'Z'";
                untilDate = [df dateFromString:val];
            } else if ([val containsString:@"T"]) {
                df.timeZone = eventTimeZone ?: [NSTimeZone defaultTimeZone];
                df.dateFormat = @"yyyyMMdd'T'HHmmss";
                untilDate = [df dateFromString:val];
            } else {
                df.timeZone = eventTimeZone ?: [NSTimeZone defaultTimeZone];
                df.dateFormat = @"yyyyMMdd'T'HHmmss";
                NSString *eodStr = [NSString stringWithFormat:@"%@T235959", val];
                untilDate = [df dateFromString:eodStr];
            }

            if (!untilDate) {
                if (outError) *outError = [NSString stringWithFormat:@"Invalid UNTIL date format: %@", val];
                return nil;
            }
            recurrenceEnd = [EKRecurrenceEnd recurrenceEndWithEndDate:untilDate];
        } else if ([key isEqualToString:@"COUNT"]) {
            NSInteger countVal = [val integerValue];
            if (countVal <= 0) {
                if (outError) *outError = [NSString stringWithFormat:@"COUNT must be a positive integer: %@", val];
                return nil;
            }
            recurrenceEnd = [EKRecurrenceEnd recurrenceEndWithOccurrenceCount:countVal];
        } else {
            if (outError) *outError = [NSString stringWithFormat:@"Unsupported recurrence property: %@", key];
            return nil;
        }
    }

    if (!hasFreq) {
        if (outError) *outError = @"Recurrence rule is missing required FREQ property.";
        return nil;
    }

    return [[EKRecurrenceRule alloc]
        initRecurrenceWithFrequency:frequency
                           interval:interval
                      daysOfTheWeek:daysOfTheWeek.count > 0 ? daysOfTheWeek : nil
                     daysOfTheMonth:nil
                    monthsOfTheYear:nil
                     weeksOfTheYear:nil
                      daysOfTheYear:nil
                       setPositions:nil
                                end:recurrenceEnd];
}

int calendar_apple_create_event(
    const char *title,
    double start_epoch,
    double end_epoch,
    int is_all_day,
    const char *location,
    const char *notes,
    const char *url,
    const char *recurrence_rule,
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
        if (recurrence_rule && strlen(recurrence_rule) > 0) {
            NSString *rruleStr = [NSString stringWithUTF8String:recurrence_rule];
            NSString *parseErr = nil;
            NSTimeZone *eventTz = event.timeZone ?: [NSTimeZone defaultTimeZone];
            EKRecurrenceRule *rule = parse_rrule_string(rruleStr, eventTz, &parseErr);
            if (!rule) {
                if (out_error) {
                    *out_error = create_c_string(parseErr ?: @"Invalid recurrence rule specified.");
                }
                return -1;
            }
            [event addRecurrenceRule:rule];
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
