# Heroinn application icon assets

These assets are generated from the repository-owned source
`image/heroinn-mark.svg`. No third-party artwork or external brand asset is
used.

Source SHA-256:

```text
3288921fac28cdf0a335ea773a46fceca55fdc1700b4a23cba5b267dc7d04aa4  image/heroinn-mark.svg
```

The assets were generated on the Linux test VM with ImageMagick 6.9.12-98
Q16:

```sh
workdir="$(mktemp -d)"
for size in 16 24 32 48 64 128 256; do
    convert -background none -density 384 image/heroinn-mark.svg \
        -resize "${size}x${size}" "PNG32:${workdir}/heroinn-${size}.png"
done

convert "${workdir}/heroinn-16.png" "${workdir}/heroinn-24.png" \
    "${workdir}/heroinn-32.png" "${workdir}/heroinn-48.png" \
    "${workdir}/heroinn-64.png" "${workdir}/heroinn-128.png" \
    "${workdir}/heroinn-256.png" heroinn/resources/heroinn.ico
cp "${workdir}/heroinn-128.png" heroinn/resources/heroinn-128.png
```

`heroinn.ico` contains 16, 24, 32, 48, 64, 128, and 256 pixel RGBA frames.
The 128 pixel PNG is decoded at runtime by `heroinn/src/icon.rs` and used as the
eframe window icon. The 128 pixel frame is selected because X11's single
`_NET_WM_ICON` request cannot carry a 256 pixel RGBA image with its two-cardinal
header. The Windows executable embeds the multi-size ICO through
`heroinn/resources/heroinn.rc`; the resource build is gated to Windows targets
by `heroinn/build.rs`.
