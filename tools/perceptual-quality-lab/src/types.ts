export type MediaKind = "image" | "audio";

export interface MediaInfo {
  path: string;
  fileName: string;
  extension: string;
  fileBytes: number;
  mediaKind: MediaKind;
  width?: number;
  height?: number;
  durationSeconds?: number;
  sampleRate?: number;
  channels?: number;
  codec?: string;
}

export interface PairInspection {
  source: MediaInfo;
  candidate: MediaInfo;
  sameMediaKind: boolean;
  formallyComparable: boolean;
  blockers: string[];
  warnings: string[];
}

export interface ThresholdResult {
  profile: "release_smoke" | "forensic_default" | "balanced_candidate";
  passed: boolean;
  blockingReason: string;
}

export interface ImageQualityReport {
  width: number;
  height: number;
  psnr: number;
  ssim: number;
  mae: number;
  p95AbsoluteDifference: number;
  maxChannelDifference: number;
  changedPixelRatio: number;
  release: ThresholdResult;
  forensic: ThresholdResult;
  balanced: ThresholdResult;
}

export interface ImageAnalysisResult {
  report: ImageQualityReport;
  sourcePreviewDataUrl: string;
  candidatePreviewDataUrl: string;
  heatmaps: {
    x1DataUrl: string;
    x4DataUrl: string;
    x16DataUrl: string;
  };
}

export interface AudioQualityReport {
  sampleRate: number;
  channels: number;
  comparedSamples: number;
  snr: number;
  peakDelta: number;
  lufsDelta: number;
  newClipping: number;
  silenceNoiseFloorDelta: number;
  perceptualDiagnosis: {
    segmentedSnr: {
      segmentSeconds: number;
      segmentCount: number;
      min: number;
      mean: number;
      max: number;
      first: number;
      middle: number;
      last: number;
      spread: number;
    };
    bandEnergy: {
      lowSignalShare: number;
      lowNoiseShare: number;
      watermarkSignalShare: number;
      watermarkNoiseShare: number;
      highSignalShare: number;
      highNoiseShare: number;
      dominantNoiseBand: string;
    };
    diagnosis: string;
  };
  release: ThresholdResult;
  forensic: ThresholdResult;
  balanced: ThresholdResult;
}

export interface AudioAnalysisResult {
  report: AudioQualityReport;
  formallyComparable: boolean;
  blockers: string[];
  warnings: string[];
  alignment: {
    sourceTrimSeconds: number;
    candidateTrimSeconds: number;
    detectedOffsetSeconds: number;
    correlationScore: number;
    commonDurationSeconds: number;
  };
  waveform: {
    source: number[];
    candidate: number[];
    difference: number[];
  };
  sourceClipPath: string;
  candidateClipPath: string;
  clipStartSeconds: number;
  clipDurationSeconds: number;
}

export interface AbxAssets {
  mediaKind: MediaKind;
  sourceAsset: string;
  candidateAsset: string;
  startSeconds: number;
  durationSeconds: number;
}

export type AbxIdentity = "source" | "candidate";
export type AbxChoice = "a" | "b";

export interface AbxTrial {
  index: number;
  a: AbxIdentity;
  b: AbxIdentity;
  x: AbxChoice;
  answer?: AbxChoice;
  confidence: number;
  perceivedDifference: string;
  notes: string;
}

export interface AbxSummary {
  correct: number;
  total: number;
  correctRate: number;
  pValue: number;
  conclusion: "no_stable_evidence" | "inconclusive" | "review_required";
}
