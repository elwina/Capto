/**
 * Shared types for the overlay-layout settings blob.
 *
 * The overlay config is persisted as JSON by the Rust backend and edited via
 * dotted-path patches (e.g. "mouseClicks.leftColor") from the React UI, so the
 * shape is intentionally loose. These types replace the previous `Record<string, any>`
 * so the UI code is checked against the fields it actually reads.
 */

export type OverlayPosition = {
  anchor?: string;
  x?: number;
  y?: number;
};

export type MouseClickOverlay = {
  enabled?: boolean;
  leftColor?: string;
  rightColor?: string;
};

export type KeystrokesOverlay = {
  enabled?: boolean;
  fontSize?: number;
  color?: string;
  background?: string;
  position?: OverlayPosition;
};

export type OverlayWebcam = {
  enabled?: boolean;
  deviceId?: string | null;
  deviceLabel?: string | null;
  width?: number;
  height?: number;
  cornerRadius?: number;
  mirrored?: boolean;
  position?: OverlayPosition;
};

export type TextOverlay = {
  id?: string;
  enabled?: boolean;
  text?: string;
  fontSize?: number;
  color?: string;
  position?: OverlayPosition;
};

export type ImageOverlay = {
  id?: string;
  enabled?: boolean;
  path?: string;
  width?: number;
  height?: number;
  opacity?: number;
  position?: OverlayPosition;
};

/** Runtime overlay-layout settings. Optional fields tolerate configs written by older app builds. */
export type OverlaysSettings = {
  mouseClicks?: MouseClickOverlay;
  keystrokes?: KeystrokesOverlay;
  webcam?: OverlayWebcam;
  texts?: TextOverlay[];
  images?: ImageOverlay[];
};
