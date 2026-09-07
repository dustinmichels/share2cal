#import "ShareViewController.h"
#import <MobileCoreServices/MobileCoreServices.h>
#import <UniformTypeIdentifiers/UniformTypeIdentifiers.h>

static NSString * const kAppGroupIdentifier = @"group.com.dustinmichels.share2cal";
static NSString * const kCustomUrlScheme = @"share2cal://share";

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
    self.statusLabel.text = @"Importing flyer to share2cal...";
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

- (void)processSharedItems {
    NSExtensionItem *firstItem = self.extensionContext.inputItems.firstObject;
    if (!firstItem || firstItem.attachments.count == 0) {
        [self finishWithError:@"No shared items found"];
        return;
    }

    // Look for image attachment
    NSArray *imageTypeIdentifiers = @[
        @"public.image",
        @"public.jpeg",
        @"public.png",
        @"public.heic",
        @"public.heif",
        @"public.tiff",
        @"com.apple.quicktime-image"
    ];

    NSItemProvider *imageProvider = nil;
    NSString *matchedTypeIdentifier = nil;

    for (NSItemProvider *provider in firstItem.attachments) {
        for (NSString *typeId in imageTypeIdentifiers) {
            if ([provider hasItemConformingToTypeIdentifier:typeId]) {
                imageProvider = provider;
                matchedTypeIdentifier = typeId;
                break;
            }
        }
        if (imageProvider) break;
    }

    if (!imageProvider) {
        // Fallback: check for file URL
        for (NSItemProvider *provider in firstItem.attachments) {
            if ([provider hasItemConformingToTypeIdentifier:@"public.file-url"]) {
                imageProvider = provider;
                matchedTypeIdentifier = @"public.file-url";
                break;
            }
        }
    }

    if (!imageProvider) {
        [self finishWithError:@"No image found in share payload"];
        return;
    }

    [imageProvider loadItemForTypeIdentifier:matchedTypeIdentifier options:nil completionHandler:^(id<NSSecureCoding> item, NSError *error) {
        if (error) {
            dispatch_async(dispatch_get_main_queue(), ^{
                [self finishWithError:error.localizedDescription];
            });
            return;
        }

        NSData *imageData = nil;
        NSString *fileExtension = @"png";
        NSString *mimeType = @"image/png";

        if ([(NSObject *)item isKindOfClass:[NSURL class]]) {
            NSURL *url = (NSURL *)item;
            NSString *ext = url.pathExtension.lowercaseString;
            if (ext.length > 0) {
                fileExtension = ext;
            }
            if ([fileExtension isEqualToString:@"jpg"] || [fileExtension isEqualToString:@"jpeg"]) {
                mimeType = @"image/jpeg";
            } else if ([fileExtension isEqualToString:@"heic"] || [fileExtension isEqualToString:@"heif"]) {
                mimeType = @"image/heic";
            } else if ([fileExtension isEqualToString:@"webp"]) {
                mimeType = @"image/webp";
            }

            imageData = [NSData dataWithContentsOfURL:url];
        } else if ([(NSObject *)item isKindOfClass:[UIImage class]]) {
            UIImage *image = (UIImage *)item;
            imageData = UIImagePNGRepresentation(image);
            if (!imageData) {
                imageData = UIImageJPEGRepresentation(image, 0.95);
                fileExtension = @"jpg";
                mimeType = @"image/jpeg";
            }
        } else if ([(NSObject *)item isKindOfClass:[NSData class]]) {
            imageData = (NSData *)item;
        }

        if (!imageData || imageData.length == 0) {
            dispatch_async(dispatch_get_main_queue(), ^{
                [self finishWithError:@"Failed to read image data"];
            });
            return;
        }

        BOOL saved = [self saveSharedImageData:imageData extension:fileExtension mimeType:mimeType];
        dispatch_async(dispatch_get_main_queue(), ^{
            if (saved) {
                [self openHostAppAndComplete];
            } else {
                [self finishWithError:@"Failed to save image to shared storage"];
            }
        });
    }];
}

- (BOOL)saveSharedImageData:(NSData *)data extension:(NSString *)ext mimeType:(NSString *)mimeType {
    NSURL *containerURL = [[NSFileManager defaultManager] containerURLForSecurityApplicationGroupIdentifier:kAppGroupIdentifier];
    if (!containerURL) {
        // Fallback: local document directory if app group is unavailable
        containerURL = [[[NSFileManager defaultManager] URLsForDirectory:NSCachesDirectory inDomains:NSUserDomainMask] firstObject];
    }

    if (!containerURL) {
        return NO;
    }

    NSURL *sharedDir = [containerURL URLByAppendingPathComponent:@"shared_images" isDirectory:YES];
    NSError *dirError = nil;
    [[NSFileManager defaultManager] createDirectoryAtURL:sharedDir withIntermediateDirectories:YES attributes:nil error:&dirError];

    NSTimeInterval timestamp = [[NSDate date] timeIntervalSince1970];
    NSString *fileName = [NSString stringWithFormat:@"shared_flyer_%ld.%@", (long)timestamp, ext];
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
        @"mime_type": mimeType,
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
    NSURL *url = [NSURL URLWithString:kCustomUrlScheme];

    // Attempt to open the host app via responder chain
    UIResponder *responder = self;
    BOOL opened = NO;
    while (responder != nil) {
        if ([responder respondsToSelector:@selector(openURL:)]) {
            [responder performSelector:@selector(openURL:) withObject:url];
            opened = YES;
            break;
        }
        responder = [responder nextResponder];
    }

    // Complete the extension request
    [self.extensionContext completeRequestReturningItems:@[] completionHandler:^(BOOL expired) {
        // Extension dismissed
    }];
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
