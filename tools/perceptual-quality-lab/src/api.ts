import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import type {
  AbxAssets,
  AudioAnalysisResult,
  ImageAnalysisResult,
  PairInspection,
} from "./types";

export function assetUrl(path: string): string {
  return convertFileSrc(path);
}

export function inspectMediaPair(sourcePath: string, candidatePath: string) {
  return invoke<PairInspection>("inspect_media_pair", { sourcePath, candidatePath });
}

export function analyzeImagePair(sourcePath: string, candidatePath: string) {
  return invoke<ImageAnalysisResult>("analyze_image_pair", { sourcePath, candidatePath });
}

export function analyzeAudioPair(
  sourcePath: string,
  candidatePath: string,
  clipStartSeconds: number,
) {
  return invoke<AudioAnalysisResult>("analyze_audio_pair", {
    sourcePath,
    candidatePath,
    clipStartSeconds,
  });
}

export function prepareAbxAssets(
  sourcePath: string,
  candidatePath: string,
  startSeconds: number,
) {
  return invoke<AbxAssets>("prepare_abx_assets", {
    sourcePath,
    candidatePath,
    startSeconds,
  });
}

export function clearLabSession() {
  return invoke<void>("clear_lab_session");
}
