# DESIGN.md - Photon

## Architecture Overview

Photon is a cross-platform (Android, Windows) file transfer application built around a strong core library written in Rust. It utilizes Tauri 2 for the frontend UI, communicating with the core Rust crate for all backend logic.

### Directory Structure

- `core/`: Standalone Rust crate containing all logic for network discovery (mDNS), encryption (Noise Protocol), transfer (chunking, BLAKE3, resume logic), and file system operations. Fully testable without UI.
- `cli/`: A command-line interface wrapping `core/` for testing and headless usage.
- `scripts/`: Build scripts (`build-android.sh`, `build-windows.ps1`).
- `.github/workflows/`: CI/CD definitions for automated release builds.
- *(Future)* `app/`: Tauri application directory (Tauri 2, React, Tailwind, Framer Motion).
- *(Future)* `app/src-tauri/`: Tauri's Rust backend wrapper linking to `core/`.
- *(Future)* `plugins/android/`: Kotlin plugin for Android-specific functionality (Foreground service, SAF, Share target, Haptics).

## Design Tokens

The app follows a modern, clean, and vibrant design aesthetic.

- **Accent Color:** Electric violet to cyan gradient (`bg-gradient-to-r from-violet-600 to-cyan-500`).
- **Typography:** Inter or system sans-serif font stack. Generous spacing and clean modern type scale.
- **Shapes:** Soft rounded corners (e.g., `rounded-2xl`, `rounded-full` for avatars).
- **Effects:** Soft glow effects around primary actionable elements (especially when discovering or transferring).

## Shared Motion Tokens

Animations are powered by Framer Motion. They should feel premium, fluid, and responsive.

### Constants

```typescript
export const MOTION_TOKENS = {
  // Durations
  duration: {
    fast: 0.2,
    normal: 0.35,
    slow: 0.5,
  },

  // Easings
  easing: {
    easeOut: [0.0, 0.0, 0.2, 1],
    easeInOut: [0.4, 0.0, 0.2, 1],
    spring: { type: "spring", stiffness: 300, damping: 25 },
    springBouncy: { type: "spring", stiffness: 400, damping: 15 },
  },

  // Transitions
  transition: {
    page: { type: "tween", ease: "easeInOut", duration: 0.3 },
    hover: { type: "spring", stiffness: 400, damping: 25 },
    popIn: { type: "spring", stiffness: 300, damping: 20 },
  }
};
```

### Motion Guidelines

- Scale transforms and opacities only.
- Throttle high-frequency updates (e.g., progress bars) to ~15fps in the UI to maintain 60fps on low-end devices.
- Respect reduced motion settings (`prefers-reduced-motion: reduce`), replacing springs with fast crossfades.
