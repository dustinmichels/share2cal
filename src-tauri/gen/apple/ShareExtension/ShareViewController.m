#import "ShareViewController.h"
#import <MobileCoreServices/MobileCoreServices.h>
#import <UniformTypeIdentifiers/UniformTypeIdentifiers.h>

static NSString * const kAppGroupIdentifier = @"group.com.dustinmichels.share2cal";
static NSString * const kCustomUrlScheme = @"share2cal://share";

static NSString *DetectExtensionFromData(NSData *data, NSString *fallbackExt) {
    if (!data || data.length < 4) return fallbackExt ?: @"jpg";
    const uint8_t *bytes = (const uint8_t *)data.bytes;

    // JPEG: FF D8 FF
    if (data.length >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF) {
        return @"jpg";
    }
    // PNG: 89 50 4E 47
    if (data.length >= 4 && bytes[0] == 0x89 && bytes[1] == 0x50 && bytes[2] == 0x4E && bytes[3] == 0x47) {
        return @"png";
    }
    // GIF: 47 49 46 38
    if (data.length >= 4 && bytes[0] == 0x47 && bytes[1] == 0x49 && bytes[2] == 0x46 && bytes[3] == 0x38) {
        return @"gif";
    }
    // WebP: RIFF ... WEBP
    if (data.length >= 12 && bytes[0] == 'R' && bytes[1] == 'I' && bytes[2] == 'F' && bytes[3] == 'F' &&
        bytes[8] == 'W' && bytes[9] == 'E' && bytes[10] == 'B' && bytes[11] == 'P') {
        return @"webp";
    }
    // HEIF / HEIC: ftyp (bytes 4-7) followed by heic / heix / hevc / hevm / mif1 / msf1
    if (data.length >= 12 && bytes[4] == 'f' && bytes[5] == 't' && bytes[6] == 'y' && bytes[7] == 'p') {
        NSString *brand = [[NSString alloc] initWithBytes:&bytes[8] length:4 encoding:NSASCIIStringEncoding];
        if (brand) {
            NSString *brandLower = brand.lowercaseString;
            if ([brandLower hasPrefix:@"hei"] || [brandLower hasPrefix:@"hev"] || [brandLower isEqualToString:@"mif1"] || [brandLower isEqualToString:@"msf1"]) {
                return @"heic";
            }
        }
    }
    // TIFF: II*. or MM.*
    if (data.length >= 4) {
        if ((bytes[0] == 'I' && bytes[1] == 'I' && bytes[2] == 0x2A && bytes[3] == 0x00) ||
            (bytes[0] == 'M' && bytes[1] == 'M' && bytes[2] == 0x00 && bytes[3] == 0x2A)) {
            return @"tiff";
        }
    }

    return fallbackExt ?: @"jpg";
}

static NSString *MimeTypeForExtension(NSString *ext) {
    NSString *e = ext.lowercaseString;
    if ([e isEqualToString:@"jpg"] || [e isEqualToString:@"jpeg"]) return @"image/jpeg";
    if ([e isEqualToString:@"png"]) return @"image/png";
    if ([e isEqualToString:@"heic"] || [e isEqualToString:@"heif"]) return @"image/heic";
    if ([e isEqualToString:@"webp"]) return @"image/webp";
    if ([e isEqualToString:@"gif"]) return @"image/gif";
    if ([e isEqualToString:@"tiff"] || [e isEqualToString:@"tif"]) return @"image/tiff";
    return @"image/jpeg";
}

@interface ShareViewController ()
@property (nonatomic, strong) UIActivityIndicatorView *spinner;
@property (nonatomic, strong) UILabel *statusLabel;
@property (nonatomic, strong) UIView *containerCard;
@end

@implementation ShareViewController

- (void)viewDidLoad {
    [super viewDidLoad];
    [self setupUI];
}

- (void)viewDidAppear:(BOOL)animated {
    [super viewDidAppear:animated];
    [self processSharedItems];
}

- (void)setupUI {
    self.view.backgroundColor = [[UIColor blackColor] colorWithAlphaComponent:0.4];

    // Card container
    self.containerCard = [[UIView alloc] init];
    self.containerCard.translatesAutoresizingMaskIntoConstraints = NO;
    self.containerCard.backgroundColor = [UIColor colorWithRed:0.12 green:0.14 blue:0.18 alpha:0.95];
    self.containerCard.layer.cornerRadius = 20;
    self.containerCard.layer.masksToBounds = YES;
    self.containerCard.layer.borderWidth = 1;
    self.containerCard.layer.borderColor = [[UIColor colorWithWhite:1.0 alpha:0.15] CGColor];
    [self.view addSubview:self.containerCard];

    // Spinner
    self.spinner = [[UIActivityIndicatorView alloc] initWithActivityIndicatorStyle:UIActivityIndicatorViewStyleLarge];
    self.spinner.translatesAutoresizingMaskIntoConstraints = NO;
    self.spinner.color = [UIColor colorWithRed:0.38 green:0.65 blue:0.98 alpha:1.0];
    [self.spinner startAnimating];
    [self.containerCard addSubview:self.spinner];

    // Status label
    self.statusLabel = [[UILabel alloc] init];
    self.statusLabel.translatesAutoresizingMaskIntoConstraints = NO;
    self.statusLabel.text = @"Importing flyer to Share2Cal...";
    self.statusLabel.textColor = [UIColor whiteColor];
    self.statusLabel.font = [UIFont systemFontOfSize:16 weight:UIFontWeightMedium];
    self.statusLabel.textAlignment = NSTextAlignmentCenter;
    self.statusLabel.numberOfLines = 0;
    [self.containerCard addSubview:self.statusLabel];

    // Constraints
    [NSLayoutConstraint activateConstraints:@[
        [self.containerCard.centerXAnchor constraintEqualToAnchor:self.view.centerXAnchor],
        [self.containerCard.centerYAnchor constraintEqualToAnchor:self.view.centerYAnchor],
        [self.containerCard.widthAnchor constraintEqualToConstant:280],
        [self.containerCard.heightAnchor constraintGreaterThanOrEqualToConstant:140],

        [self.spinner.centerXAnchor constraintEqualToAnchor:self.containerCard.centerXAnchor],
        [self.spinner.topAnchor constraintEqualToAnchor:self.containerCard.topAnchor constant:28],

        [self.statusLabel.topAnchor constraintEqualToAnchor:self.spinner.bottomAnchor constant:16],
        [self.statusLabel.leadingAnchor constraintEqualToAnchor:self.containerCard.leadingAnchor constant:16],
        [self.statusLabel.trailingAnchor constraintEqualToAnchor:self.containerCard.trailingAnchor constant:-16],
        [self.statusLabel.bottomAnchor constraintEqualToAnchor:self.containerCard.bottomAnchor constant:-24]
    ]];
}

- (NSString *)extensionForTypeIdentifier:(NSString *)typeIdentifier {
    if (!typeIdentifier) return @"jpg";
    NSString *t = typeIdentifier.lowercaseString;
    if ([t containsString:@"jpeg"] || [t containsString:@"jpg"]) return @"jpg";
    if ([t containsString:@"png"]) return @"png";
    if ([t containsString:@"heic"]) return @"heic";
    if ([t containsString:@"heif"]) return @"heif";
    if ([t containsString:@"webp"]) return @"webp";
    if ([t containsString:@"gif"]) return @"gif";
    if ([t containsString:@"tiff"] || [t containsString:@"tif"]) return @"tiff";
    return @"jpg";
}

- (void)processSharedItems {
    NSExtensionItem *firstItem = self.extensionContext.inputItems.firstObject;
    if (!firstItem || firstItem.attachments.count == 0) {
        [self finishWithError:@"No shared items found"];
        return;
    }

    // Preferred image type identifiers in order of specificity
    NSArray<NSString *> *preferredImageTypes = @[
        @"public.jpeg",
        @"public.png",
        @"public.heic",
        @"public.heif",
        @"public.tiff",
        @"com.apple.quicktime-image",
        @"public.camera-raw-image",
        @"com.adobe.raw-image",
        @"public.image",
        @"public.file-url",
        @"public.url"
    ];

    NSItemProvider *imageProvider = nil;
    NSString *matchedTypeIdentifier = nil;

    // 1. Check if any attachment explicitly registers a preferred image type
    for (NSItemProvider *provider in firstItem.attachments) {
        for (NSString *typeId in preferredImageTypes) {
            if ([provider.registeredTypeIdentifiers containsObject:typeId]) {
                imageProvider = provider;
                matchedTypeIdentifier = typeId;
                break;
            }
        }
        if (imageProvider) break;
    }

    // 2. Check conformance to preferred types
    if (!imageProvider) {
        for (NSItemProvider *provider in firstItem.attachments) {
            for (NSString *typeId in preferredImageTypes) {
                if ([provider hasItemConformingToTypeIdentifier:typeId]) {
                    imageProvider = provider;
                    matchedTypeIdentifier = typeId;
                    break;
                }
            }
            if (imageProvider) break;
        }
    }

    // 3. Fallback: check any registered type conforming to public.image
    if (!imageProvider) {
        for (NSItemProvider *provider in firstItem.attachments) {
            for (NSString *regType in provider.registeredTypeIdentifiers) {
                if ([provider hasItemConformingToTypeIdentifier:regType] && [provider hasItemConformingToTypeIdentifier:@"public.image"]) {
                    imageProvider = provider;
                    matchedTypeIdentifier = regType;
                    break;
                }
            }
            if (imageProvider) break;
        }
    }

    if (!imageProvider || !matchedTypeIdentifier) {
        [self finishWithError:@"No compatible image found in share payload"];
        return;
    }

    [self loadSharedImageFromProvider:imageProvider typeIdentifier:matchedTypeIdentifier];
}

- (void)loadSharedImageFromProvider:(NSItemProvider *)provider typeIdentifier:(NSString *)typeIdentifier {
    // Attempt 1: Load data representation directly (iOS 11+)
    if ([provider respondsToSelector:@selector(loadDataRepresentationForTypeIdentifier:completionHandler:)]) {
        [provider loadDataRepresentationForTypeIdentifier:typeIdentifier completionHandler:^(NSData *data, NSError *error) {
            if (data && data.length > 0 && !error) {
                NSString *ext = DetectExtensionFromData(data, [self extensionForTypeIdentifier:typeIdentifier]);
                NSString *mime = MimeTypeForExtension(ext);
                [self handleLoadedImageData:data extension:ext mimeType:mime];
                return;
            }

            // Fallback to loadItemForTypeIdentifier
            [self loadItemFallbackFromProvider:provider typeIdentifier:typeIdentifier];
        }];
        return;
    }

    [self loadItemFallbackFromProvider:provider typeIdentifier:typeIdentifier];
}

- (void)loadItemFallbackFromProvider:(NSItemProvider *)provider typeIdentifier:(NSString *)typeIdentifier {
    [provider loadItemForTypeIdentifier:typeIdentifier options:nil completionHandler:^(id<NSSecureCoding> item, NSError *error) {
        NSData *imageData = nil;
        NSString *inferredExt = [self extensionForTypeIdentifier:typeIdentifier];
        NSString *fileExtension = inferredExt ?: @"jpg";
        NSString *mimeType = @"image/jpeg";

        if ([(NSObject *)item isKindOfClass:[NSURL class]]) {
            NSURL *url = (NSURL *)item;
            BOOL isSecurityScoped = [url startAccessingSecurityScopedResource];
            imageData = [NSData dataWithContentsOfURL:url];
            if (isSecurityScoped) {
                [url stopAccessingSecurityScopedResource];
            }

            NSString *urlExt = url.pathExtension.lowercaseString;
            if (urlExt.length > 0) {
                fileExtension = urlExt;
            }
        } else if ([(NSObject *)item isKindOfClass:[UIImage class]]) {
            UIImage *image = (UIImage *)item;
            imageData = UIImageJPEGRepresentation(image, 0.92);
            if (!imageData) {
                imageData = UIImagePNGRepresentation(image);
                fileExtension = @"png";
            } else {
                fileExtension = @"jpg";
            }
        } else if ([(NSObject *)item isKindOfClass:[NSData class]]) {
            imageData = (NSData *)item;
        }

        if (imageData && imageData.length > 0) {
            fileExtension = DetectExtensionFromData(imageData, fileExtension);
            mimeType = MimeTypeForExtension(fileExtension);
            [self handleLoadedImageData:imageData extension:fileExtension mimeType:mimeType];
            return;
        }

        // Final fallback: try loadObjectOfClass:[UIImage class]
        if ([provider canLoadObjectOfClass:[UIImage class]]) {
            [provider loadObjectOfClass:[UIImage class] completionHandler:^(id<NSItemProviderReading> object, NSError *objError) {
                if ([(NSObject *)object isKindOfClass:[UIImage class]]) {
                    UIImage *image = (UIImage *)object;
                    NSData *imgData = UIImageJPEGRepresentation(image, 0.92);
                    if (!imgData) {
                        imgData = UIImagePNGRepresentation(image);
                    }
                    if (imgData && imgData.length > 0) {
                        NSString *ext = DetectExtensionFromData(imgData, @"jpg");
                        NSString *mime = MimeTypeForExtension(ext);
                        [self handleLoadedImageData:imgData extension:ext mimeType:mime];
                        return;
                    }
                }

                dispatch_async(dispatch_get_main_queue(), ^{
                    [self finishWithError:error ? error.localizedDescription : @"Failed to read image data from Photos"];
                });
            }];
            return;
        }

        dispatch_async(dispatch_get_main_queue(), ^{
            [self finishWithError:error ? error.localizedDescription : @"Failed to read image data"];
        });
    }];
}

- (void)handleLoadedImageData:(NSData *)data extension:(NSString *)ext mimeType:(NSString *)mimeType {
    BOOL saved = [self saveSharedImageData:data extension:ext mimeType:mimeType];
    dispatch_async(dispatch_get_main_queue(), ^{
        if (saved) {
            [self openHostAppAndComplete];
        } else {
            [self finishWithError:@"Failed to save image to shared storage"];
        }
    });
}

- (BOOL)saveSharedImageData:(NSData *)data extension:(NSString *)ext mimeType:(NSString *)mimeType {
    if (!data || data.length == 0) return NO;

    NSURL *containerURL = [[NSFileManager defaultManager] containerURLForSecurityApplicationGroupIdentifier:kAppGroupIdentifier];
    if (!containerURL) {
        // Fallback: local caches directory if app group is unavailable
        containerURL = [[[NSFileManager defaultManager] URLsForDirectory:NSCachesDirectory inDomains:NSUserDomainMask] firstObject];
        if (containerURL) {
            containerURL = [containerURL URLByAppendingPathComponent:@"com.dustinmichels.share2cal" isDirectory:YES];
        }
    }

    if (!containerURL) {
        return NO;
    }

    NSURL *sharedDir = [containerURL URLByAppendingPathComponent:@"shared_images" isDirectory:YES];
    NSError *dirError = nil;
    [[NSFileManager defaultManager] createDirectoryAtURL:sharedDir withIntermediateDirectories:YES attributes:nil error:&dirError];

    NSTimeInterval timestamp = [[NSDate date] timeIntervalSince1970];
    NSString *fileExt = (ext && ext.length > 0) ? ext : @"jpg";
    NSString *fileName = [NSString stringWithFormat:@"shared_flyer_%ld.%@", (long)timestamp, fileExt];
    NSURL *fileURL = [sharedDir URLByAppendingPathComponent:fileName];

    NSError *writeError = nil;
    BOOL success = [data writeToURL:fileURL options:NSDataWritingAtomic error:&writeError];
    if (!success || writeError) {
        return NO;
    }

    // Write manifest JSON
    NSDictionary *manifest = @{
        @"file_name": fileName,
        @"file_path": fileURL.path,
        @"mime_type": mimeType ?: @"image/jpeg",
        @"size_bytes": @(data.length),
        @"timestamp": @((long long)timestamp),
        @"source": @"ios_share_extension"
    };

    NSURL *manifestURL = [sharedDir URLByAppendingPathComponent:@"pending_share.json"];
    NSData *jsonData = [NSJSONSerialization dataWithJSONObject:manifest options:NSJSONWritingPrettyPrinted error:nil];
    if (jsonData) {
        [jsonData writeToURL:manifestURL options:NSDataWritingAtomic error:nil];
    }

    return YES;
}

- (void)openHostAppAndComplete {
    self.statusLabel.text = @"Opening Share2Cal...";
    self.spinner.hidden = YES;

    NSURL *url = [NSURL URLWithString:kCustomUrlScheme];

    __block BOOL completionCalled = NO;
    void (^dismissExtensionBlock)(void) = ^{
        if (completionCalled) return;
        completionCalled = YES;
        dispatch_async(dispatch_get_main_queue(), ^{
            [self.extensionContext completeRequestReturningItems:@[] completionHandler:nil];
        });
    };

    // 1. Try NSExtensionContext openURL:completionHandler:
    SEL openURLWithCompletion = NSSelectorFromString(@"openURL:completionHandler:");
    if ([self.extensionContext respondsToSelector:openURLWithCompletion]) {
        NSMethodSignature *signature = [self.extensionContext methodSignatureForSelector:openURLWithCompletion];
        if (signature) {
            NSInvocation *invocation = [NSInvocation invocationWithMethodSignature:signature];
            [invocation setTarget:self.extensionContext];
            [invocation setSelector:openURLWithCompletion];
            [invocation setArgument:&url atIndex:2];
            void (^openCompletion)(BOOL) = ^(BOOL success) {
                dismissExtensionBlock();
            };
            [invocation setArgument:&openCompletion atIndex:3];
            [invocation invoke];

            // Fallback timeout in case the completion handler block is not called by iOS
            dispatch_after(dispatch_time(DISPATCH_TIME_NOW, (int64_t)(0.6 * NSEC_PER_SEC)), dispatch_get_main_queue(), ^{
                dismissExtensionBlock();
            });
            return;
        }
    }

    // 2. Try openURL: on self.extensionContext directly
    SEL openURLSel = NSSelectorFromString(@"openURL:");
    if ([self.extensionContext respondsToSelector:openURLSel]) {
        #pragma clang diagnostic push
        #pragma clang diagnostic ignored "-Warc-performSelector-leaks"
        [self.extensionContext performSelector:openURLSel withObject:url];
        #pragma clang diagnostic pop
        dispatch_after(dispatch_time(DISPATCH_TIME_NOW, (int64_t)(0.4 * NSEC_PER_SEC)), dispatch_get_main_queue(), ^{
            dismissExtensionBlock();
        });
        return;
    }

    // 3. Try UIResponder chain traversal
    UIResponder *responder = self;
    while (responder != nil) {
        if ([responder respondsToSelector:openURLSel]) {
            #pragma clang diagnostic push
            #pragma clang diagnostic ignored "-Warc-performSelector-leaks"
            [responder performSelector:openURLSel withObject:url];
            #pragma clang diagnostic pop
            break;
        }
        responder = [responder nextResponder];
    }

    // 4. Try UIApplication sharedApplication via reflection
    Class uiAppClass = NSClassFromString(@"UIApplication");
    if (uiAppClass) {
        SEL sharedAppSel = NSSelectorFromString(@"sharedApplication");
        if ([uiAppClass respondsToSelector:sharedAppSel]) {
            #pragma clang diagnostic push
            #pragma clang diagnostic ignored "-Warc-performSelector-leaks"
            id app = [uiAppClass performSelector:sharedAppSel];
            if (app && [app respondsToSelector:openURLSel]) {
                [app performSelector:openURLSel withObject:url];
            }
            #pragma clang diagnostic pop
        }
    }

    // 5. Dismiss extension after brief delay
    dispatch_after(dispatch_time(DISPATCH_TIME_NOW, (int64_t)(0.5 * NSEC_PER_SEC)), dispatch_get_main_queue(), ^{
        dismissExtensionBlock();
    });
}

- (void)finishWithError:(NSString *)message {
    self.statusLabel.text = message ?: @"Failed to import flyer";
    self.spinner.hidden = YES;

    dispatch_after(dispatch_time(DISPATCH_TIME_NOW, (int64_t)(1.5 * NSEC_PER_SEC)), dispatch_get_main_queue(), ^{
        [self.extensionContext cancelRequestWithError:[NSError errorWithDomain:@"com.dustinmichels.share2cal.ShareExtension"
                                                                          code:1
                                                                      userInfo:@{NSLocalizedDescriptionKey: message ?: @"Unknown error"}]];
    });
}

@end
