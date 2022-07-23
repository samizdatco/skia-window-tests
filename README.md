# Windowing Experiments for Skia Canvas

This repository contains a few proof-of-concept prototypes attempting to use a combination of [`rust-skia`][rust_skia], [`winit`][winit], and various graphics backends to render directly to the screen. The approaches that work can be incorporated into [`skia-canvas`][skia_canvas] to allow for the creation of windows, handling of user input, etc.

The first step is to be able to render something/anything from a Skia context to the screen. Ideally each of these approaches would be able to
  1. Allow multiple windows to be created and closed (using `winit` to get access to window handles and control the event loop)
  2. Work on multiple platforms, ideally using the same backend that's used for generating image files
  3. Potentially be able to run each window in a separate thread (if that actually makes a performance difference)

### Platform Considerations

It's tempting to try to use OpenGL for all three platforms, but macOS support has been lagging for years now and Apple officially [deprecated it][gl_deprecated] as of 10.14. As a result, the macOS version should use Metal, and the real question is whether Linux and Windows can share a graphics backend (presumably either GL or Vulkan).

### Current Status

The example in the [`metal`][metal] subdirectory seems to be fully functional, and, if nothing else, is a good demonstration of what the others are trying to accomplish. The [`gl`][gl] and [`vulkan`][vulkan] subprojects are both broken in different ways (detailed below) and I could really use the help of folks with more GPU experience as I try to get one or both of them running.

## [Metal][metal]

```console
cd metal
cargo run
```

This version actually works! The windows can be resized, minimized, maximized, and closed without interfering with one another. When the final window is closed, the application terminates.


<img alt="metal windows" src="/metal/screenshot.png" width="400">

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

## [Vulkan][vulkan]

```console
cd vulkan
cargo run
```

In theory, Vulkan seems like the most promising approach for targeting Windows & Linux given how well it works for [generating bitmaps](https://github.com/samizdatco/skia-canvas/blob/gpu/src/gpu/vulkan.rs) and how much less global state it's saddled with compared to GL. However it's also just staggeringly low-level and setting up the necessary graphics pipelines requires a rather deep understanding of its inner workings. The [`skulpin`][skulpin] project has already built a renderer that integrates Vulkan and Skia, but it's currently non-functional on my development machine (a Macintosh) so it's difficult for me to test whether it would be a good solution for the other platforms.

Only the current pre-release version of [`ash`][ash] (the Vulkan bindings used by most rust projects) runs on ARM Macs so I've attempted to update `skulpin` to work with that. A few releases ago, `skulpin` moved its internals to a much larger (and broader) framework called rafx which was too heavyweight for my purposes, so this directory instead contains a [fork](vulkan/skulpin-renderer) of the [last pre-rafx version][skulpin_fork] of `skulpin`.

I've managed to successfully create all the basic pieces (instance, device, shaders, swap-chains, etc.) without error, but the rendering pipeline is running into what look like synchronization problems when it actually runs. In particular, an error claiming that it expected a layout of `VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL` but received `VK_IMAGE_LAYOUT_UNDEFINED`. 

```
Setting up skia backend context with queue family index 0
Surface format: SurfaceFormatKHR { format: B8G8R8A8_UNORM, color_space: SRGB_NONLINEAR }
Swapchain extents chosen by surface capabilities (800 600)
Extents: Extent2D { width: 800, height: 600 }
Available present modes: [FIFO, IMMEDIATE]
Preferred present modes: [Fifo]
Present mode: FIFO
Created 3 swapchain images with initial size (800, 600).
Creating command pool with queue family index 0
Create skia surfaces with extent: Extent2D { width: 800, height: 600 }
Validation Error: [ UNASSIGNED-CoreValidation-DrawState-InvalidImageLayout ] Object 0: handle = 0x136f29bd8, type = VK_OBJECT_TYPE_COMMAND_BUFFER; | MessageID = 0x4dae5635 | vkQueueSubmit(): pSubmits[0].pCommandBuffers[0] command buffer VkCommandBuffer 0x136f29bd8[] expects VkImage 0x3a6cbb0000000025[] (subresource: aspectMask 0x1 array layer 0, mip level 0) to be in layout VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL--instead, current layout is VK_IMAGE_LAYOUT_UNDEFINED.
Validation Error: [ UNASSIGNED-CoreValidation-DrawState-InvalidImageLayout ] Object 0: handle = 0x136f29d18, type = VK_OBJECT_TYPE_COMMAND_BUFFER; | MessageID = 0x4dae5635 | vkQueueSubmit(): pSubmits[0].pCommandBuffers[0] command buffer VkCommandBuffer 0x136f29d18[] expects VkImage 0xa43473000000002d[] (subresource: aspectMask 0x1 array layer 0, mip level 0) to be in layout VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL--instead, current layout is VK_IMAGE_LAYOUT_UNDEFINED.
Validation Error: [ UNASSIGNED-CoreValidation-DrawState-InvalidImageLayout ] Object 0: handle = 0x136f29e58, type = VK_OBJECT_TYPE_COMMAND_BUFFER; | MessageID = 0x4dae5635 | vkQueueSubmit(): pSubmits[0].pCommandBuffers[0] command buffer VkCommandBuffer 0x136f29e58[] expects VkImage 0xa808d50000000033[] (subresource: aspectMask 0x1 array layer 0, mip level 0) to be in layout VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL--instead, current layout is VK_IMAGE_LAYOUT_UNDEFINED.
```

Googling for this turns up *many* other people debugging memory barriers that aren't correctly balanced, but I'm reaching the limit of my current understanding of Vulkan as I try to figure out what's going wrong. If you've got a better sense of how this should be working, start by taking a look at the contents of [`skia_renderpass.rs`](https://github.com/samizdatco/skia-window-tests/blob/main/vulkan/skulpin-renderer/src/skia_renderpass.rs) and the [Renderer](https://github.com/samizdatco/skia-window-tests/blob/e7f673a70147caee81e8da5d9cd208a508f67923/vulkan/skulpin-renderer/src/renderer.rs#L215) struct that [invokes](https://github.com/samizdatco/skia-window-tests/blob/e7f673a70147caee81e8da5d9cd208a508f67923/vulkan/skulpin-renderer/src/renderer.rs#L439) it and let me know if you see anything suspicious…

<img alt="vulkan window not rendering content" src="/vulkan/screenshot.png" width="400">


[gl]: gl
[metal]: metal
[vulkan]: vulkan
[skulpin]: https://github.com/aclysma/skulpin
[ash]: https://github.com/ash-rs/ash
[skulpin_fork]: https://github.com/aclysma/skulpin/tree/4a2ae275fc42e9a6fcbf12aa1b9d713c34bc5db2
[skia_canvas]: https://github.com/samizdatco/skia-canvas
[rust_skia]: https://github.com/rust-skia/rust-skia
[gl_window]: https://github.com/rust-skia/rust-skia/blob/master/skia-safe/examples/gl-window/main.rs
[gl_multiwindow]: https://github.com/rust-windowing/glutin/blob/master/glutin_examples/examples/multiwindow.rs
[gl_deprecated]: https://arstechnica.com/features/2018/09/macos-10-14-mojave-the-ars-technica-review/12/
[gl_context_tracker]: https://github.com/rust-windowing/glutin/blob/4e55db7e65a7bbd08d32a5b26fd7827b4aaf4211/glutin_examples/examples/support/mod.rs#L134
[glutin]: https://github.com/rust-windowing/glutin
[winit]: https://github.com/rust-windowing/winit