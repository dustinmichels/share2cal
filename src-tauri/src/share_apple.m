#import <Foundation/Foundation.h>
#import "share_apple.h"

static NSURL* get_container_url(const char* group_id) {
    NSURL *containerURL = nil;
    if (group_id != NULL && strlen(group_id) > 0) {
        NSString *groupIdStr = [NSString stringWithUTF8String:group_id];
        containerURL = [[NSFileManager defaultManager] containerURLForSecurityApplicationGroupIdentifier:groupIdStr];
    }

    if (!containerURL) {
        // Fallback to caches directory if group container is not accessible (e.g. running on macOS desktop or unsigned simulator)
        containerURL = [[[NSFileManager defaultManager] URLsForDirectory:NSCachesDirectory inDomains:NSUserDomainMask] firstObject];
        if (containerURL) {
            containerURL = [containerURL URLByAppendingPathComponent:@"com.dustinmichels.share2cal" isDirectory:YES];
        }
    }

    return containerURL;
}

char* get_app_group_pending_share_json(const char* group_id) {
    @autoreleasepool {
        NSURL *containerURL = get_container_url(group_id);
        if (!containerURL) {
            return NULL;
        }

        NSURL *sharedDir = [containerURL URLByAppendingPathComponent:@"shared_images" isDirectory:YES];
        NSURL *manifestURL = [sharedDir URLByAppendingPathComponent:@"pending_share.json"];

        if ([[NSFileManager defaultManager] fileExistsAtPath:manifestURL.path]) {
            NSError *error = nil;
            NSString *jsonStr = [NSString stringWithContentsOfURL:manifestURL encoding:NSUTF8StringEncoding error:&error];
            if (jsonStr && !error) {
                return strdup([jsonStr UTF8String]);
            }
        }

        // If manifest doesn't exist, check if there are any image files in shared_images
        if ([[NSFileManager defaultManager] fileExistsAtPath:sharedDir.path]) {
            NSError *error = nil;
            NSArray<NSString *> *files = [[NSFileManager defaultManager] contentsOfDirectoryAtPath:sharedDir.path error:&error];
            if (files && files.count > 0) {
                NSString *latestFile = nil;
                NSDate *latestDate = nil;

                for (NSString *file in files) {
                    if ([file isEqualToString:@"pending_share.json"]) continue;
                    NSString *filePath = [sharedDir.path stringByAppendingPathComponent:file];
                    NSDictionary *attrs = [[NSFileManager defaultManager] attributesOfItemAtPath:filePath error:nil];
                    NSDate *modDate = attrs[NSFileModificationDate];

                    if (!latestDate || (modDate && [modDate compare:latestDate] == NSOrderedDescending)) {
                        latestDate = modDate;
                        latestFile = file;
                    }
                }

                if (latestFile) {
                    NSString *filePath = [sharedDir.path stringByAppendingPathComponent:latestFile];
                    NSDictionary *attrs = [[NSFileManager defaultManager] attributesOfItemAtPath:filePath error:nil];
                    unsigned long long fileSize = [attrs[NSFileSize] unsignedLongLongValue];
                    NSTimeInterval timestamp = latestDate ? [latestDate timeIntervalSince1970] : [[NSDate date] timeIntervalSince1970];

                    NSString *ext = latestFile.pathExtension.lowercaseString;
                    NSString *mime = @"image/png";
                    if ([ext isEqualToString:@"jpg"] || [ext isEqualToString:@"jpeg"]) {
                        mime = @"image/jpeg";
                    } else if ([ext isEqualToString:@"heic"] || [ext isEqualToString:@"heif"]) {
                        mime = @"image/heic";
                    } else if ([ext isEqualToString:@"webp"]) {
                        mime = @"image/webp";
                    }

                    NSDictionary *syntheticManifest = @{
                        @"file_name": latestFile,
                        @"file_path": filePath,
                        @"mime_type": mime,
                        @"size_bytes": @(fileSize),
                        @"timestamp": @((long long)timestamp),
                        @"source": @"ios_share_container_scan"
                    };

                    NSData *data = [NSJSONSerialization dataWithJSONObject:syntheticManifest options:0 error:nil];
                    if (data) {
                        NSString *str = [[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding];
                        return strdup([str UTF8String]);
                    }
                }
            }
        }

        return NULL;
    }
}

char* get_app_group_shared_image_path(const char* group_id) {
    @autoreleasepool {
        NSURL *containerURL = get_container_url(group_id);
        if (!containerURL) {
            return NULL;
        }

        NSURL *sharedDir = [containerURL URLByAppendingPathComponent:@"shared_images" isDirectory:YES];
        NSURL *manifestURL = [sharedDir URLByAppendingPathComponent:@"pending_share.json"];

        if ([[NSFileManager defaultManager] fileExistsAtPath:manifestURL.path]) {
            NSData *data = [NSData dataWithContentsOfURL:manifestURL];
            if (data) {
                NSDictionary *json = [NSJSONSerialization JSONObjectWithData:data options:0 error:nil];
                if (json && [json isKindOfClass:[NSDictionary class]]) {
                    NSString *filePath = json[@"file_path"];
                    if (filePath && [[NSFileManager defaultManager] fileExistsAtPath:filePath]) {
                        return strdup([filePath UTF8String]);
                    }
                }
            }
        }

        // Fallback: check sharedDir for any image files
        if ([[NSFileManager defaultManager] fileExistsAtPath:sharedDir.path]) {
            NSArray<NSString *> *files = [[NSFileManager defaultManager] contentsOfDirectoryAtPath:sharedDir.path error:nil];
            for (NSString *file in files) {
                if ([file isEqualToString:@"pending_share.json"]) continue;
                NSString *path = [sharedDir.path stringByAppendingPathComponent:file];
                if ([[NSFileManager defaultManager] fileExistsAtPath:path]) {
                    return strdup([path UTF8String]);
                }
            }
        }

        return NULL;
    }
}

bool clear_app_group_shared_data(const char* group_id) {
    @autoreleasepool {
        NSURL *containerURL = get_container_url(group_id);
        if (!containerURL) {
            return false;
        }

        NSURL *sharedDir = [containerURL URLByAppendingPathComponent:@"shared_images" isDirectory:YES];
        if (![[NSFileManager defaultManager] fileExistsAtPath:sharedDir.path]) {
            return true;
        }

        NSError *error = nil;
        [[NSFileManager defaultManager] removeItemAtURL:sharedDir error:&error];
        return ![[NSFileManager defaultManager] fileExistsAtPath:sharedDir.path];
    }
}

bool save_app_group_shared_image(
    const char* group_id,
    const uint8_t* bytes,
    size_t len,
    const char* filename,
    const char* mime_type
) {
    @autoreleasepool {
        if (!bytes || len == 0) {
            return false;
        }

        NSURL *containerURL = get_container_url(group_id);
        if (!containerURL) {
            return false;
        }

        NSURL *sharedDir = [containerURL URLByAppendingPathComponent:@"shared_images" isDirectory:YES];
        NSError *dirError = nil;
        [[NSFileManager defaultManager] createDirectoryAtURL:sharedDir withIntermediateDirectories:YES attributes:nil error:&dirError];

        NSString *fName = nil;
        if (filename != NULL && strlen(filename) > 0) {
            fName = [NSString stringWithUTF8String:filename];
        } else {
            fName = [NSString stringWithFormat:@"shared_image_%ld.png", (long)[[NSDate date] timeIntervalSince1970]];
        }

        NSString *mime = nil;
        if (mime_type != NULL && strlen(mime_type) > 0) {
            mime = [NSString stringWithUTF8String:mime_type];
        } else {
            mime = @"image/png";
        }

        NSURL *fileURL = [sharedDir URLByAppendingPathComponent:fName];
        NSData *data = [NSData dataWithBytes:bytes length:len];
        NSError *writeError = nil;
        if (![data writeToURL:fileURL options:NSDataWritingAtomic error:&writeError]) {
            return false;
        }

        NSTimeInterval timestamp = [[NSDate date] timeIntervalSince1970];
        NSDictionary *manifest = @{
            @"file_name": fName,
            @"file_path": fileURL.path,
            @"mime_type": mime,
            @"size_bytes": @(len),
            @"timestamp": @((long long)timestamp),
            @"source": @"share_apple_bridge"
        };

        NSURL *manifestURL = [sharedDir URLByAppendingPathComponent:@"pending_share.json"];
        NSData *jsonData = [NSJSONSerialization dataWithJSONObject:manifest options:NSJSONWritingPrettyPrinted error:nil];
        if (jsonData) {
            [jsonData writeToURL:manifestURL options:NSDataWritingAtomic error:nil];
        }

        return YES;
    }
}

void free_share_string(char* str) {
    if (str != NULL) {
        free(str);
    }
}
