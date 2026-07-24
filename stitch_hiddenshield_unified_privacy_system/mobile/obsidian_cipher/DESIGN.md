---
name: Obsidian Cipher
colors:
  surface: '#0e1513'
  surface-dim: '#0e1513'
  surface-bright: '#333b39'
  surface-container-lowest: '#09100e'
  surface-container-low: '#161d1b'
  surface-container: '#1a211f'
  surface-container-high: '#242b2a'
  surface-container-highest: '#2f3634'
  on-surface: '#dde4e1'
  on-surface-variant: '#bacac5'
  inverse-surface: '#dde4e1'
  inverse-on-surface: '#2b3230'
  outline: '#859490'
  outline-variant: '#3c4a46'
  surface-tint: '#3cddc7'
  primary: '#57f1db'
  on-primary: '#003731'
  primary-container: '#2dd4bf'
  on-primary-container: '#00574d'
  inverse-primary: '#006b5f'
  secondary: '#ffb95f'
  on-secondary: '#472a00'
  secondary-container: '#ee9800'
  on-secondary-container: '#5b3800'
  tertiary: '#ffd1aa'
  on-tertiary: '#4b2800'
  tertiary-container: '#ffac5a'
  on-tertiary-container: '#744000'
  error: '#ffb4ab'
  on-error: '#690005'
  error-container: '#93000a'
  on-error-container: '#ffdad6'
  primary-fixed: '#62fae3'
  primary-fixed-dim: '#3cddc7'
  on-primary-fixed: '#00201c'
  on-primary-fixed-variant: '#005047'
  secondary-fixed: '#ffddb8'
  secondary-fixed-dim: '#ffb95f'
  on-secondary-fixed: '#2a1700'
  on-secondary-fixed-variant: '#653e00'
  tertiary-fixed: '#ffdcc0'
  tertiary-fixed-dim: '#ffb875'
  on-tertiary-fixed: '#2d1600'
  on-tertiary-fixed-variant: '#6b3b00'
  background: '#0e1513'
  on-background: '#dde4e1'
  surface-variant: '#2f3634'
typography:
  headline-lg:
    fontFamily: Inter
    fontSize: 24px
    fontWeight: '600'
    lineHeight: 32px
    letterSpacing: -0.01em
  headline-md:
    fontFamily: Inter
    fontSize: 20px
    fontWeight: '600'
    lineHeight: 28px
    letterSpacing: -0.01em
  headline-sm:
    fontFamily: Inter
    fontSize: 16px
    fontWeight: '600'
    lineHeight: 24px
  body-lg:
    fontFamily: Inter
    fontSize: 16px
    fontWeight: '400'
    lineHeight: 24px
  body-md:
    fontFamily: Inter
    fontSize: 14px
    fontWeight: '400'
    lineHeight: 20px
  body-sm:
    fontFamily: Inter
    fontSize: 13px
    fontWeight: '400'
    lineHeight: 18px
  label-lg:
    fontFamily: Inter
    fontSize: 14px
    fontWeight: '500'
    lineHeight: 16px
    letterSpacing: 0.05em
  label-md:
    fontFamily: Inter
    fontSize: 12px
    fontWeight: '500'
    lineHeight: 16px
  label-sm:
    fontFamily: Inter
    fontSize: 11px
    fontWeight: '600'
    lineHeight: 12px
    letterSpacing: 0.02em
rounded:
  sm: 0.125rem
  DEFAULT: 0.25rem
  md: 0.375rem
  lg: 0.5rem
  xl: 0.75rem
  full: 9999px
spacing:
  unit: 4px
  container-margin: 24px
  gutter: 16px
  padding-xs: 4px
  padding-sm: 8px
  padding-md: 16px
  padding-lg: 24px
---

## Brand & Style

The design system is engineered for high-stakes privacy and digital asset protection. The brand personality is "Silent Sentinel"—a quiet, authoritative presence that prioritizes utility over decoration. It targets enterprise users and security-conscious professionals who require a sense of absolute control and stability.

The design style is **Corporate / Modern** with a lean toward **Minimalism**. It utilizes a "Task-First" philosophy, where the UI recedes to highlight the user's workflow. Visual flourishes like gradients or organic shapes are strictly excluded to maintain an atmosphere of serious, enterprise-grade reliability. The interface uses restrained surfaces and high-contrast text to ensure immediate legibility in operational environments.

## Colors

This design system operates exclusively in a **Dark** color mode to reduce eye strain during prolonged technical tasks and to evoke a sense of "secure room" privacy. 

The palette is anchored by a sharp Teal (#2DD4BF) for primary actions, representing digital precision. A Copper/Amber (#F59E0B) is used sparingly for secondary highlights and security-related warnings, providing a warm, high-visibility contrast against the cool dark backgrounds. The background hierarchy uses a deep Navy (#0F172A) for the base canvas and a lighter Slate (#1E293B) for interactive surfaces. Borders and dividers are kept subtle using a muted Slate (#334155) to maintain structure without clutter.

## Typography

The typography system uses **Inter** for its neutral, systematic, and highly readable characteristics. To maintain an "operational tool" feel, titles are kept compact; we intentionally avoid oversized display fonts to maximize information density.

Hierarchy is established through weight and letter spacing rather than sheer scale. Labels often use a subtle uppercase treatment with increased letter spacing to distinguish metadata from content. For mobile devices, the headline sizes remain consistent as they are already optimized for smaller viewport constraints.

## Layout & Spacing

The layout follows a strict **Fixed Grid** logic for desktop (12 columns) and a fluid 4-column structure for mobile. A 4px baseline grid ensures vertical rhythm across all components.

Layouts should prioritize data density. Margins are consistent at 24px, and gutters are 16px to maintain a compact but legible separation of concerns. In data-heavy views, use a "Sidebar + Main Stage" pattern to allow quick navigation between security modules.

## Elevation & Depth

In this design system, depth is communicated through **Tonal Layers** and low-contrast outlines rather than heavy shadows. The UI is intentionally flat to emphasize the "Shield" metaphor—solid and impenetrable.

1.  **Level 0 (Base):** #0F172A (Background deep).
2.  **Level 1 (Cards/Panels):** #1E293B with a 1px border of #334155.
3.  **Level 2 (Modals/Popovers):** #1E293B with a subtle, tight shadow (0px 4px 12px rgba(0,0,0,0.5)) to provide separation.

There are no backdrop blurs or glassmorphism effects. Surfaces are opaque and definitive.

## Shapes

The shape language is rigid and professional. We use a **Soft (0.25rem / 4px)** base roundedness for most UI elements like input fields and buttons. Larger containers like cards use an 8px (0.5rem) radius to soften the edges of the application slightly without appearing playful. This precision in geometry reinforces the software’s technical and secure nature.

## Components

### Buttons & Controls
- **Primary Button:** Solid #2DD4BF background with #0F172A text. 4px border radius.
- **Secondary Button:** Ghost style with #334155 border and white/off-white text.
- **Checkboxes/Radios:** Square-ish (2px radius), using the Primary Teal for the checked state.

### Status Chips
Status chips are critical for the "operational" feel. They use a low-opacity background of their status color with a high-contrast text and a solid 2px left-border or a small leading dot.
- **Ready:** Teal text/border.
- **Locked/Warning:** Amber text/border.
- **Failed:** Red text/border.

### Progress Indicators
Progress bars are slim (4px height) with a secondary label placed above or below. Every progress indicator must include **explicit stage text** (e.g., "Stage 2 of 4: Encrypting Metadata...") to keep the user informed.

### Cards & Lists
Cards use the #1E293B surface with the 8px radius. Lists should use thin #334155 dividers between items. Interaction states (hover) should be indicated by a subtle background shift to #2D3748, never by a shadow increase.

### Input Fields
Inputs use a dark #0F172A fill to inset them into the #1E293B card surface, creating a "carved" effect that feels secure and tactile. Focus states are marked by a 1px solid #2DD4BF ring.