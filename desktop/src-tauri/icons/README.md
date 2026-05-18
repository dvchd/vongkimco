# Icon assets

Files in this directory are **generated** from the master SVG at
[`../../../backend/static/app-icon.svg`](../../../backend/static/app-icon.svg)
and are gitignored — do not commit them.

The SVG lives under `backend/static/` so it is also served by the backend at
`/static/app-icon.svg` (used by the admin web UI). Single source of truth for
both the desktop app icon and the web logo.

Regenerate locally:

```bash
cd desktop
npm install     # if not done
npm run icons:build
```

CI does this automatically before each release build.

Generated artifacts include:

- Cross-platform: `32x32.png`, `64x64.png`, `128x128.png`, `128x128@2x.png`, `icon.png`
- Windows: `icon.ico`, `Square*Logo.png`, `StoreLogo.png`
- macOS: `icon.icns`
- iOS: `ios/AppIcon-*.png`
- Android: `android/mipmap-*/ic_launcher*.png`
