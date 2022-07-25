# Windowing Experiments for Skia Canvas

This repository contains a few proof-of-concept prototypes attempting to use a combination of [`rust-skia`][rust_skia], [`winit`][winit], and various graphics backends to render directly to the screen. The approaches that work can be incorporated into [`skia-canvas`][skia_canvas] to allow for the creation of windows, handling of user input, etc.

The first step is to be able to render something/anything from a Skia context to the screen. Ideally each of these approaches would be able to
  1. Allow multiple windows to be created and closed (using `winit` to get access to window handles and control the event loop)
  2. Work on multiple platforms, ideally using the same backend that's used for generating image files
  3. Potentially be able to run each window in a separate thread (if that actually makes a performance difference)

### Platform Considerations

It's tempting to try to use OpenGL for all three platforms, but macOS support has been lagging for years now and Apple officially [deprecated it][gl_deprecated] as of 10.14. As a result, the macOS version should use Metal, and the real question is whether Linux and Windows can share a graphics backend (presumably either GL or Vulkan).

### Current Status

The example in the [`metal`][metal] subdirectory seems to be fully functional, and, if nothing else, is a good demonstration of what the others are trying to accomplish. The [`vulkan`][vulkan] subproject has also been confirmed to work on Macs that have installed the MoltenVK libraries, but still needs to be tested on Linux & Windows. The [`gl`][gl] subproject is still having problems with crosstalk between windows (detailed below) and I could really use the help of folks with more GPU experience as I try to get that sorted out.

## [Metal][metal]

```console
cd metal
cargo run
```

This version actually works! The windows can be resized, minimized, maximized, and closed without interfering with one another. When the final window is closed, the application terminates.

<img alt="metal windows" src="/metal/screenshot.png" width="400">

## [Vulkan][vulkan]

```console
cd vulkan
cargo run
```

This is now running properly on the Mac (which is useful for development purposes even if it won't be actively used in `skia-canvas`). The trick is to use an older version of the ‘MoltenVK’ [Vulkan SDK][molten_sdk] on macOS since the 1.3.x lineage is either tripping up Skia or the Rafx library that's bridging between Skia and the display. I had luck with the [version 1.2.189 installer][molten_sdk_download].

After running the installer, the easiest way to make it visible to the dynamic linker is to run the `install_vulkan.py` script in the sdk folder. You can also leave everything as-is and set some environment variables along the lines of:
```sh
export VULKAN_SDK="/path/to/vulkan-sdk/macOS"
export DYLD_LIBRARY_PATH="$VULKAN_SDK/lib:$DYLD_LIBRARY_PATH"
export VK_ICD_FILENAMES="$VULKAN_SDK/share/vulkan/icd.d/MoltenVK_icd.json"
export VK_LAYER_PATH="$VULKAN_SDK/share/vulkan/explicit_layer.d"
export PATH="$VULKAN_SDK/bin:$PATH
```

On Linux & Windows this isn't necessary but I could use some feedback on what actually **is** required to get the demo to run. This article has tips for [setting up Vulkan on linux](https://linuxconfig.org/install-and-test-vulkan-on-linux), and my understanding is that Windows graphics card drivers include everything necessary, but confirmation on both these fronts would be quite helpful.


<img alt="vulkan working like a charm" src="/vulkan/screenshot.png" width="400">

## [OpenGL][gl]

```console
cd gl
cargo run
```

This version uses [`glutin`][glutin] (a wrapper around [`winit`][winit] that handles the creation of OpenGL contexts) and is adapted from the `skia-safe` [`gl-window`][gl_window] example. It runs acceptably when there's only a single window and GL Context, but I've been less successful in attempting to get it to support multiple windows.

For that, I adapted the `glutin` [`multiwindow`][gl_multiwindow] example and attempted to slim down its fairly elaborate [`ContextTracker`][gl_context_tracker] module (whose main purpose is to ensure that the appropriate GL context is made ‘current’ before being drawn to).

When first launched, everything seems to be fine—each window is playing its independent animation. But as soon as a window is resized (and needs to [recreate the surface](https://github.com/samizdatco/skia-window-tests/blob/e7f673a70147caee81e8da5d9cd208a508f67923/gl/src/main.rs#L125-L159) it's drawing to) it's clear that the different windows’ contexts are not being correctly made ‘current’/‘non-current’ by the [`get_current`](https://github.com/samizdatco/skia-window-tests/blob/e7f673a70147caee81e8da5d9cd208a508f67923/gl/src/contexts.rs#L108) helper. As you resize one window, content from other windows starts overdrawing it.

A related bug can be triggered by closing a window: you'll see the content of that window ‘hop’ to one of the remaining windows.

<img alt="gl windows working at first" src="/gl/screenshot-1.png" width="360">&nbsp;<img alt="gl windows glitching after resize" src="/gl/screenshot-2.png" width="360">



[gl]: gl
[metal]: metal
[vulkan]: vulkan
[skulpin]: https://github.com/aclysma/skulpin
[ash]: https://github.com/ash-rs/ash
[skulpin_fork]: https://github.com/aclysma/skulpin/tree/4a2ae275fc42e9a6fcbf12aa1b9d713c34bc5db2
[skia_canvas]: https://github.com/samizdatco/skia-canvas
[rust_skia]: https://github.com/rust-skia/rust-skia
[molten_sdk]: https://vulkan.lunarg.com/sdk/home#mac
[molten_sdk_download]: https://sdk.lunarg.com/sdk/download/1.2.189.0/mac/vulkansdk-macos-1.2.189.0.dmg
[gl_window]: https://github.com/rust-skia/rust-skia/blob/master/skia-safe/examples/gl-window/main.rs
[gl_multiwindow]: https://github.com/rust-windowing/glutin/blob/master/glutin_examples/examples/multiwindow.rs
[gl_deprecated]: https://arstechnica.com/features/2018/09/macos-10-14-mojave-the-ars-technica-review/12/
[gl_context_tracker]: https://github.com/rust-windowing/glutin/blob/4e55db7e65a7bbd08d32a5b26fd7827b4aaf4211/glutin_examples/examples/support/mod.rs#L134
[glutin]: https://github.com/rust-windowing/glutin
[winit]: https://github.com/rust-windowing/winit