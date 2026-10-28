# Running on iPhone

A dev build streams the frontend from Vite on your Mac. A release build bundles it, so the app keeps working after you unplug.

## Dev build on a device

```sh
bun run tauri ios dev --host --open
```

Xcode opens with the `share2cal_iOS` scheme. Pick your phone as the run destination and press Run. The phone loads the UI from your Mac's LAN address, so both must be on the same network.

## Standalone install

1. **Build and open Xcode.**

   ```sh
   bun run tauri ios build --open
   ```

2. **Choose the phone.** In the toolbar destination menu, pick your connected iPhone rather than _Any iOS Device_ or a simulator.
3. **Switch the scheme to Release.** Product › Scheme › Edit Scheme… (<kbd>⌘</kbd> <kbd><</kbd>), select **Run**, open the **Info** tab and set **Build Configuration** to **Release**.
4. **Check signing for both targets.** Select the `share2cal` project, then confirm **Signing & Capabilities** for `share2cal_iOS` _and_ `ShareExtension`. Both need your team and the same App Group, `group.com.dustinmichels.share2cal`.
5. **Run** (<kbd>⌘</kbd> <kbd>R</kbd>). Xcode compiles, signs, installs and launches the app.

::: warning Both targets must share the App Group
The Share Extension and the app exchange images through the App Group container. If the group is missing from either entitlements file, sharing appears to work but the app never sees the image. Each target's entitlements live in `src-tauri/gen/apple/ShareExtension/ShareExtension.entitlements` and `src-tauri/gen/apple/share2cal_iOS/share2cal_iOS.entitlements`.
:::

## First time on a new phone

### Enable Developer Mode

iOS 16 and later refuse to launch apps installed from Xcode until Developer Mode is on. The toggle stays hidden until the phone has been paired with Xcode, so do the steps in order.

1. **Connect the phone.** Plug it into your Mac with a cable and unlock it.
2. **Trust the Mac.** When the phone asks _Trust This Computer?_, tap **Trust** and enter your passcode. If you missed the prompt, unplug and reconnect.
3. **Pair with Xcode.** Open Xcode, then Window › Devices and Simulators (<kbd>⇧</kbd> <kbd>⌘</kbd> <kbd>2</kbd>). Select the phone in the sidebar. Xcode may show _Preparing device for development_ while it copies debug symbols; wait for it to finish. Selecting the phone as the run destination and pressing Run has the same effect.
4. **Turn on the toggle.** On the phone, open Settings › Privacy & Security, scroll to the bottom to the **Security** section and tap **Developer Mode**. Switch it on.
5. **Restart.** iOS warns that Developer Mode reduces security. Tap **Restart**.
6. **Confirm after reboot.** Unlock the phone. When the _Turn On Developer Mode?_ alert appears, tap **Turn On** and enter your passcode.

Back in Devices and Simulators, the phone should no longer report that Developer Mode is disabled. You only do this once per phone; it stays on until you switch it off or erase the device.

::: tip Developer Mode toggle is missing
- The phone has not been paired yet. Repeat steps 1–3, then close and reopen Settings.
- The phone is managed by a work or school profile (Settings › General › VPN & Device Management). Your organisation may block Developer Mode; use a personal device.
- The phone runs iOS 15 or earlier. Those versions have no Developer Mode and need no toggle.
:::

### Trust the developer certificate

A free Apple ID (Personal Team) signs apps with a certificate the phone does not trust by default. If launching the app shows _Untrusted Developer_, open Settings › General › VPN & Device Management, tap your Apple ID under **Developer App**, tap **Trust** and confirm. Paid Developer Program accounts usually skip this step.

### Optional: run without the cable

After the first wired pairing, Xcode 15 and later connect over Wi‑Fi automatically: while the phone and Mac share a network, the phone stays in the run destination menu without the cable, marked with a globe icon. Xcode 14 and earlier need **Connect via network** checked for the phone in Devices and Simulators.

## Trying the share flow

1. Open Photos, choose a flyer and tap Share.
2. Pick **Share2Cal** from the share sheet. The extension accepts up to 10 images or files.
3. The extension saves the image, opens `share2cal://share` and dismisses itself.
4. The app checks for a pending share when it launches, gains focus or becomes visible, and starts scanning straight away.

The details are in [Getting images in](/architecture/sharing).
