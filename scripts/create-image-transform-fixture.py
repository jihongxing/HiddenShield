from pathlib import Path
import sys

from PIL import Image


def main() -> None:
    if len(sys.argv) < 5:
        raise SystemExit(
            "usage: create-image-transform-fixture.py <input> <output> <operation> <value>"
        )
    input_path = Path(sys.argv[1])
    output_path = Path(sys.argv[2])
    operation = sys.argv[3]
    value = sys.argv[4]

    with Image.open(input_path) as source:
        if operation == "rotate":
            angle = int(value)
            transpose = {
                90: Image.Transpose.ROTATE_270,
                180: Image.Transpose.ROTATE_180,
                270: Image.Transpose.ROTATE_90,
            }[angle]
            output = source.transpose(transpose)
            output.save(output_path, format="PNG")
        elif operation == "scale":
            factor = float(value)
            size = (
                max(1, round(source.width * factor)),
                max(1, round(source.height * factor)),
            )
            output = source.resize(size, Image.Resampling.LANCZOS)
            output.save(output_path, format="PNG")
        elif operation == "jpeg":
            source.convert("RGB").save(
                output_path,
                format="JPEG",
                quality=int(value),
                subsampling=0,
                optimize=False,
            )
        elif operation == "webp":
            source.convert("RGB").save(
                output_path,
                format="WEBP",
                quality=int(value),
                method=4,
            )
        else:
            raise SystemExit(f"unsupported operation: {operation}")


if __name__ == "__main__":
    main()
