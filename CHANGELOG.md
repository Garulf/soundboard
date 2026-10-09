# Changelog

## 0.1.0 (2026-10-09)


### Features

* **app:** add soundboard window, sound editor, settings and tray ([6d19016](https://github.com/Garulf/soundboard/commit/6d1901637789cb1b03ff9fac860dec139ee94a15))
* **core:** add bus pump and rate adapter ([b32b0f1](https://github.com/Garulf/soundboard/commit/b32b0f12b0c4f47ba5146d8ddf5ddd08e9a52aca))
* **core:** add controller and command bus ([34ced24](https://github.com/Garulf/soundboard/commit/34ced2404e1d8ce65be23812b4fe5938d0755284))
* **core:** add library model and store ([ac485da](https://github.com/Garulf/soundboard/commit/ac485daee78d2d7f54d393e48a2144ef811ec755))
* **core:** add real-time mixer with mic passthrough input ([1333284](https://github.com/Garulf/soundboard/commit/1333284a1a0a39e62383face45db7bcacb191949))
* **core:** decode clips and measure loudness ([5cd9e4d](https://github.com/Garulf/soundboard/commit/5cd9e4dc481b87d8a31742c86783383c0a4f2d39))
* **core:** give new sounds distinct colors ([5276bff](https://github.com/Garulf/soundboard/commit/5276bff0b3d212d902f6952c18a65f18df982468))
* **platform:** accept egui key names in accelerators ([26affe3](https://github.com/Garulf/soundboard/commit/26affe32f8f49bd062493dde6aca43350328997b))
* **platform:** add global hotkey providers ([0789e7f](https://github.com/Garulf/soundboard/commit/0789e7fab5a5a7d0f498fcf86b22e866acc9edac))
* **platform:** add PipeWire backend with virtual mic ([bd0e64f](https://github.com/Garulf/soundboard/commit/bd0e64f1d376535bb7df2f5c9e9b49bdf3c2e6d4))
* **platform:** add WASAPI backend for virtual cables ([f2a5ea5](https://github.com/Garulf/soundboard/commit/f2a5ea562b55d60cea4ed7025537d8d66e744dd2))
* **remote:** add HTTP remote control with phone page ([4207852](https://github.com/Garulf/soundboard/commit/4207852740189119db8629639e602e565981a30e))


### Bug Fixes

* **core:** absorb clock drift between output devices ([4686368](https://github.com/Garulf/soundboard/commit/4686368815b8b06065aabab0f6886b2eab723649))
* **core:** make concurrent imports of identical files safe ([821be9b](https://github.com/Garulf/soundboard/commit/821be9b571e408937f5f719a35c149652c19c9ca))
* **core:** save only finished imports and retry failed saves ([f01a509](https://github.com/Garulf/soundboard/commit/f01a509bb38cfb513f00744419eff9bf83f89848))
* **platform:** recover audio backends after errors and device changes ([cf45a4d](https://github.com/Garulf/soundboard/commit/cf45a4d3aa25cdc47a5c14b8c603e3fe3cf60809))
* **platform:** report refused hotkeys per binding on Wayland ([8f2c80d](https://github.com/Garulf/soundboard/commit/8f2c80d71110fd64cb48a7c59ebf07b598cee33e))
* resolve internal crates through the workspace so version bumps build ([331170d](https://github.com/Garulf/soundboard/commit/331170d106fedc3067c49e6be8d059a8e8cadc6b))
