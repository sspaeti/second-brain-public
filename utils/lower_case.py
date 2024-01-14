from pandoc.types import *


link_index_path = "assets/indices/linkIndex.json"
content_index = "assets/indices/contentIndex.json"


def convert_to_lower_case(file_path: str):
    with open(file_path, "r") as f:
        content = f.read()
        content = content.lower()
        with open(file_path, "w") as f:
            f.write(content)


if __name__ == "__main__":
    convert_to_lower_case(link_index_path)
    # convert_to_lower_case(content_index)
