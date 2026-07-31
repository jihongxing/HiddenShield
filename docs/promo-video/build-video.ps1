param(
  [string]$Voice = "zh-CN-XiaoxiaoNeural",
  [string]$Rate = "-2%",
  [string]$Pitch = "-2Hz"
)

$ErrorActionPreference = "Stop"
$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$outputRoot = Join-Path $repoRoot "output\promo-video"
$sceneImageDir = Join-Path $outputRoot "scenes"
$voiceDir = Join-Path $outputRoot "voice-neural"
$segmentDir = Join-Path $outputRoot "segments"
$tempDir = Join-Path $outputRoot "temp"

New-Item -ItemType Directory -Force -Path $sceneImageDir, $voiceDir, $segmentDir, $tempDir | Out-Null

$sceneFile = Join-Path $PSScriptRoot "scenes.json"
$scenes = Get-Content -LiteralPath $sceneFile -Raw -Encoding UTF8 | ConvertFrom-Json

Write-Host "Rendering scene images..."
& node (Join-Path $PSScriptRoot "render-scenes.mjs")
if ($LASTEXITCODE -ne 0) { throw "Scene rendering failed." }

Write-Host "Generating neural voice narration..."
& python (Join-Path $PSScriptRoot "generate-neural-voice.py") `
  --scenes $sceneFile `
  --output $voiceDir `
  --voice $Voice `
  "--rate=$Rate" `
  "--pitch=$Pitch"
if ($LASTEXITCODE -ne 0) { throw "Neural voice generation failed." }

$sceneMetadata = @()
for ($index = 0; $index -lt $scenes.Count; $index++) {
  $sceneNumber = "{0:D2}" -f ($index + 1)
  $scene = $scenes[$index]
  $voicePath = Join-Path $voiceDir "$sceneNumber-$($scene.id).mp3"

  $speechDuration = [double](& ffprobe -v error -show_entries format=duration -of default=noprint_wrappers=1:nokey=1 $voicePath)
  $sceneDuration = [Math]::Round($speechDuration + 0.75, 3)
  $sceneMetadata += [pscustomobject]@{
    number = $sceneNumber
    id = $scene.id
    speechDuration = $speechDuration
    duration = $sceneDuration
    image = Join-Path $sceneImageDir "$sceneNumber-$($scene.id).png"
    voice = $voicePath
  }
}

$sceneMetadata | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $outputRoot "scene-timing.json") -Encoding UTF8

$videoList = Join-Path $tempDir "video-list.txt"
$audioList = Join-Path $tempDir "audio-list.txt"
Set-Content -LiteralPath $videoList -Value "" -Encoding ASCII
Set-Content -LiteralPath $audioList -Value "" -Encoding ASCII

foreach ($item in $sceneMetadata) {
  $videoPath = Join-Path $segmentDir "$($item.number)-$($item.id).mp4"
  $audioPath = Join-Path $segmentDir "$($item.number)-$($item.id).wav"
  $fadeOutStart = [Math]::Max(0, $item.duration - 0.45)

  & ffmpeg -y -loglevel error `
    -loop 1 -framerate 30 -i $item.image `
    -t $item.duration `
    -vf "scale=2048:1152,zoompan=z='min(zoom+0.00022,1.055)':x='iw/2-(iw/zoom/2)':y='ih/2-(ih/zoom/2)':d=1:s=1920x1080:fps=30,fade=t=in:st=0:d=0.45,fade=t=out:st=${fadeOutStart}:d=0.45,format=yuv420p" `
    -an -c:v libx264 -preset medium -crf 17 -pix_fmt yuv420p -r 30 $videoPath
  if ($LASTEXITCODE -ne 0) { throw "Video segment failed: $($item.id)" }

  & ffmpeg -y -loglevel error `
    -i $item.voice `
    -af "adelay=260|260,apad=pad_dur=0.75,aresample=48000" `
    -t $item.duration -c:a pcm_s16le $audioPath
  if ($LASTEXITCODE -ne 0) { throw "Audio segment failed: $($item.id)" }

  Add-Content -LiteralPath $videoList -Value "file '$($videoPath.Replace('\','/'))'" -Encoding ASCII
  Add-Content -LiteralPath $audioList -Value "file '$($audioPath.Replace('\','/'))'" -Encoding ASCII
}

$silentVideo = Join-Path $tempDir "promo-silent.mp4"
$narration = Join-Path $tempDir "narration.wav"
& ffmpeg -y -loglevel error -f concat -safe 0 -i $videoList -c copy $silentVideo
if ($LASTEXITCODE -ne 0) { throw "Video concatenation failed." }
& ffmpeg -y -loglevel error -f concat -safe 0 -i $audioList -c copy $narration
if ($LASTEXITCODE -ne 0) { throw "Narration concatenation failed." }

$totalDuration = [double](& ffprobe -v error -show_entries format=duration -of default=noprint_wrappers=1:nokey=1 $silentVideo)
$music = Join-Path $tempDir "ambient.wav"
$musicFadeOut = [Math]::Max(0, $totalDuration - 4)

& ffmpeg -y -loglevel error `
  -f lavfi -i "sine=frequency=110:sample_rate=48000:duration=$totalDuration" `
  -f lavfi -i "sine=frequency=164.81:sample_rate=48000:duration=$totalDuration" `
  -f lavfi -i "sine=frequency=220:sample_rate=48000:duration=$totalDuration" `
  -filter_complex "[0:a]volume=0.020[a0];[1:a]volume=0.012,tremolo=f=0.12:d=0.35[a1];[2:a]volume=0.006,tremolo=f=0.10:d=0.25[a2];[a0][a1][a2]amix=inputs=3:normalize=0,lowpass=f=1200,afade=t=in:st=0:d=3,afade=t=out:st=${musicFadeOut}:d=4[a]" `
  -map "[a]" -c:a pcm_s16le $music
if ($LASTEXITCODE -ne 0) { throw "Ambient track generation failed." }

$finalPath = Join-Path $outputRoot "HiddenShield-宣传片-拟人中文配音-16x9.mp4"
& ffmpeg -y -loglevel error `
  -i $silentVideo -i $narration -i $music `
  -filter_complex "[1:a]volume=1.15[n];[2:a]volume=0.85[m];[n][m]amix=inputs=2:duration=longest:normalize=0,alimiter=limit=0.95,volume=0.89[a]" `
  -map 0:v -map "[a]" `
  -c:v copy -c:a aac -b:a 192k -movflags +faststart -shortest $finalPath
if ($LASTEXITCODE -ne 0) { throw "Final mux failed." }

$previewPath = Join-Path $outputRoot "HiddenShield-宣传片-拟人中文配音-预览-720p.mp4"
& ffmpeg -y -loglevel error -i $finalPath -vf "scale=1280:720" -c:v libx264 -preset fast -crf 25 -c:a aac -b:a 128k -movflags +faststart $previewPath
if ($LASTEXITCODE -ne 0) { throw "Preview encode failed." }

Write-Host "Final video: $finalPath"
Write-Host "Preview video: $previewPath"
Write-Host "Duration: $totalDuration seconds"
