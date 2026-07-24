import { readFileSync } from 'node:fs';

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

const sources = {
  desktopCommand: readFileSync('src-tauri/src/commands/public_metadata.rs', 'utf8'),
  tauriLib: readFileSync('src-tauri/src/lib.rs', 'utf8'),
  desktopApi: readFileSync('src/lib/tauri-api.ts', 'utf8'),
  desktopVault: readFileSync('src/views/VaultView.vue', 'utf8'),
  protocolDoc: readFileSync('docs/公开权利信号与训练许可扫描协议设计.md', 'utf8'),
  packageJson: readFileSync('package.json', 'utf8'),
  tauriCargo: readFileSync('src-tauri/Cargo.toml', 'utf8'),
  runtimeQa: readFileSync('scripts/verify-public-metadata-embedded-image-runtime-qa.mjs', 'utf8'),
  avRuntimeQa: readFileSync('scripts/verify-public-metadata-embedded-av-runtime-qa.mjs', 'utf8'),
  androidRuntimeQa: readFileSync('scripts/verify-android-public-metadata-embed-runtime-qa.mjs', 'utf8'),
  androidClickQa: readFileSync(
    'scripts/verify-android-public-metadata-embed-click-qa.mjs',
    'utf8',
  ),
  watermarkQaBin: readFileSync('watermark-core/src/bin/protected_copy_file_flow_qa.rs', 'utf8'),
  desktopQaBin: readFileSync('src-tauri/examples/public_metadata_embed_qa.rs', 'utf8'),
  mobilePublicMetadataEmbedder: readFileSync(
    'mobile_app/lib/features/public_rights/public_metadata_embedder.dart',
    'utf8',
  ),
  mobileEmbedderTest: readFileSync('mobile_app/test/public_metadata_embedder_test.dart', 'utf8'),
  mobileVault: readFileSync('mobile_app/lib/features/vault/vault_page.dart', 'utf8'),
  mobileWidgetTest: readFileSync('mobile_app/test/widget_test.dart', 'utf8'),
  mobileEmbedderQaTool: readFileSync(
    'mobile_app/tool/public_metadata_embed_runtime_qa.dart',
    'utf8',
  ),
  mobileEmbedderClickQaTool: readFileSync(
    'mobile_app/tool/public_metadata_embed_click_qa.dart',
    'utf8',
  ),
};

assert(
  sources.desktopCommand.includes('export_public_rights_embedded_image') &&
    sources.tauriLib.includes('commands::public_metadata::export_public_rights_embedded_image'),
  'desktop must expose an internal local command for embedded public metadata image export',
);

assert(
  sources.desktopCommand.includes('embed_png_xmp') &&
    sources.desktopCommand.includes('PNG_ITXT_KEYWORD') &&
    sources.desktopCommand.includes('embed_jpeg_xmp') &&
    sources.desktopCommand.includes('JPEG_XMP_NAMESPACE') &&
    sources.desktopCommand.includes('embed_wav_public_metadata') &&
    sources.desktopCommand.includes('embed_mp4_public_metadata') &&
    sources.desktopCommand.includes('hsPM') &&
    sources.desktopCommand.includes('uuid') &&
    sources.tauriCargo.includes('c2pa') &&
    sources.desktopCommand.includes('Builder') &&
    sources.desktopCommand.includes('Reader') &&
    sources.desktopCommand.includes('EphemeralSigner') &&
    sources.desktopCommand.includes('HIDDENSHIELD_C2PA_SIGN_CERT_PEM') &&
    sources.desktopCommand.includes('c2pa_manifest_status') &&
    sources.desktopCommand.includes('c2pa_format_for_public_metadata_format') &&
    sources.desktopCommand.includes('"audio/wav"') &&
    sources.desktopCommand.includes('"video/mp4"') &&
    sources.desktopCommand.includes('ephemeral_development_certificate_not_publicly_trusted'),
  'desktop embed command must write real PNG iTXt, JPEG APP1, C2PA signed image/audio/video manifests, WAV RIFF chunk, and MP4 uuid metadata bytes',
);

assert(
  sources.desktopCommand.includes('legalConclusion') &&
    sources.desktopCommand.includes('公开元数据不能声明 legalConclusion=true') &&
    sources.desktopCommand.includes('creator_declaration_registry_snapshot_not_legal_advice_public_metadata_copy'),
  'embedded metadata export must preserve the non-legal-conclusion boundary',
);

assert(
  sources.desktopCommand.includes('metadata_uid != record.watermark_uid') &&
    sources.desktopCommand.includes('已阻断嵌入导出'),
  'embedded metadata export must block registry/local watermarkUid mismatch',
);

assert(
  sources.desktopApi.includes('exportPublicRightsEmbeddedImage') &&
    sources.desktopVault.includes('导出嵌入元数据图片副本') &&
    sources.desktopVault.includes('fetchPublicRightsMetadata') &&
    sources.desktopVault.includes('exportPublicRightsEmbeddedImage'),
  'desktop vault detail must wire embedded image export through registry metadata fetch',
);

assert(
  !sources.tauriLib.includes('/v1/enterprise/') &&
    !sources.desktopCommand.includes('/v1/enterprise/'),
  'public metadata embedding work must not open external Enterprise API routes',
);

assert(
    sources.packageJson.includes('rights:metadata-embed-runtime-qa') &&
    sources.packageJson.includes('rights:metadata-embed-av-runtime-qa') &&
    sources.packageJson.includes('rights:metadata-embed-android-runtime-qa') &&
    sources.packageJson.includes('rights:metadata-embed-android-click-qa') &&
    sources.runtimeQa.includes('generate-image') &&
    sources.runtimeQa.includes('/v1/watermark-ids/reserve') &&
    sources.runtimeQa.includes('/v1/sync/events:batch') &&
    sources.runtimeQa.includes('/metadata') &&
    sources.runtimeQa.includes('hasWatermarkUid') &&
    sources.runtimeQa.includes('hasManifestHash') &&
    sources.runtimeQa.includes('hasLegalConclusionFalse') &&
    sources.androidClickQa.includes('tool/public_metadata_embed_click_qa.dart') &&
    sources.androidClickQa.includes('uiautomator') &&
    sources.androidClickQa.includes('导出嵌入元数据图片副本') &&
    sources.androidClickQa.includes('byteContains') &&
    sources.androidClickQa.includes('legalConclusion="false"') &&
    sources.avRuntimeQa.includes('createWav') &&
    sources.avRuntimeQa.includes('createMp4') &&
    sources.avRuntimeQa.includes('ffmpeg') &&
    sources.avRuntimeQa.includes('hasC2paActiveManifest') &&
    sources.avRuntimeQa.includes('c2paSignerStatus') &&
    sources.avRuntimeQa.includes('hasSignedManifestHash') &&
    sources.avRuntimeQa.includes('legalConclusion=false'),
  'runtime QA must cover real backend metadata export plus byte-level embedded image and official audio/video C2PA active manifest checks',
);

assert(
  sources.mobilePublicMetadataEmbedder.includes('embedPublicRightsMetadataInImage') &&
    sources.mobilePublicMetadataEmbedder.includes('PublicMetadataImageFormat.png') &&
    sources.mobilePublicMetadataEmbedder.includes('PublicMetadataImageFormat.jpeg') &&
    sources.mobilePublicMetadataEmbedder.includes('iTXt') &&
    sources.mobilePublicMetadataEmbedder.includes('0xE1') &&
    sources.mobilePublicMetadataEmbedder.includes('legalConclusion=true') &&
    sources.mobilePublicMetadataEmbedder.includes(
      'publicRightsEmbeddedImageExportRequiresFileMessage',
    ) &&
    sources.mobileEmbedderTest.includes('PNG embedding writes iTXt XMP') &&
    sources.mobileEmbedderTest.includes('JPEG embedding writes APP1 XMP') &&
    sources.mobileVault.includes('FilePicker.pickFiles') &&
    sources.mobileVault.includes("allowedExtensions: const ['png', 'jpg', 'jpeg']") &&
    sources.mobileVault.includes('detectPublicMetadataImageFormat') &&
    sources.mobileVault.includes('embedPublicRightsMetadataInImage') &&
    sources.mobileVault.includes('checkEmbeddedPublicMetadataBytes') &&
    sources.mobileVault.includes('publicRightsEmbeddedImageExportLabel') &&
    sources.mobileVault.includes('publicRightsEmbeddedImageExportUnavailableMessage') &&
    sources.mobileVault.includes('重新选择 PNG / JPEG 保护副本') &&
    sources.mobileEmbedderQaTool.includes('RustWatermarkBridge') &&
    sources.mobileEmbedderQaTool.includes('fetchPublicRightsMetadata') &&
    sources.mobileEmbedderQaTool.includes('checkEmbeddedPublicMetadataBytes') &&
    sources.mobileEmbedderClickQaTool.includes("ValueKey('qa-export-embedded-image')") &&
    sources.mobileEmbedderClickQaTool.includes('版权库详情') &&
    sources.mobileEmbedderClickQaTool.includes('公开权利信号') &&
    sources.mobileEmbedderClickQaTool.includes('embedPublicRightsMetadataInImage') &&
    sources.mobileEmbedderClickQaTool.includes('checkEmbeddedPublicMetadataBytes') &&
    sources.androidRuntimeQa.includes('tool/public_metadata_embed_runtime_qa.dart') &&
    sources.androidRuntimeQa.includes('hasWatermarkUid') &&
    sources.androidRuntimeQa.includes('hasManifestHash') &&
    sources.androidRuntimeQa.includes('hasLegalConclusionFalse'),
  'Android must have a reusable Dart PNG/JPEG embedded metadata helper plus byte-level runtime QA using registry metadata',
);

assert(
    sources.watermarkQaBin.includes('"generate-image"') &&
    sources.watermarkQaBin.includes('--watermark-uid') &&
    sources.desktopQaBin.includes('build_xmp_packet') &&
    sources.desktopQaBin.includes('embed_c2pa_signed_manifest') &&
    sources.desktopQaBin.includes('verify_c2pa_active_manifest') &&
    sources.desktopQaBin.includes('c2pa_format_for_public_metadata_format') &&
    sources.desktopQaBin.includes('riff_hsPM_plus_c2pa') &&
    sources.desktopQaBin.includes('bmff_uuid_plus_c2pa') &&
    sources.desktopQaBin.includes('embed_png_xmp') &&
    sources.desktopQaBin.includes('embed_jpeg_xmp'),
  'runtime QA bins must generate protected PNG/JPEG copies, write C2PA manifests, and reuse desktop embedding logic',
);

console.log('public metadata embed contract passed');
