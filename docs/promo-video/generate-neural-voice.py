import argparse
import asyncio
import json
from pathlib import Path

import edge_tts


async def generate_voice(
    scenes_path: Path,
    output_dir: Path,
    voice: str,
    rate: str,
    pitch: str,
    volume: str,
) -> None:
    scenes = json.loads(scenes_path.read_text(encoding="utf-8"))
    output_dir.mkdir(parents=True, exist_ok=True)

    for index, scene in enumerate(scenes, start=1):
        scene_number = f"{index:02d}"
        output_path = output_dir / f"{scene_number}-{scene['id']}.mp3"
        communicator = edge_tts.Communicate(
            text=scene["narration"],
            voice=voice,
            rate=rate,
            pitch=pitch,
            volume=volume,
        )
        await communicator.save(str(output_path))
        print(f"Generated {output_path.name}")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--scenes", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--voice", default="zh-CN-XiaoxiaoNeural")
    parser.add_argument("--rate", default="-2%")
    parser.add_argument("--pitch", default="-2Hz")
    parser.add_argument("--volume", default="+0%")
    args = parser.parse_args()

    asyncio.run(
        generate_voice(
            scenes_path=args.scenes,
            output_dir=args.output,
            voice=args.voice,
            rate=args.rate,
            pitch=args.pitch,
            volume=args.volume,
        )
    )


if __name__ == "__main__":
    main()
