import os
from datetime import datetime
import shutil
from pathlib import Path
import re
import glob

import pandoc
from pandoc.types import *
import frontmatter

# loop through directly recursively through the directory and find all the .md files
# then loop through the files and find all the publish notes
# then loop through file backwards and find hashtag "#publish"
# if file found, move it to public folder

git = os.getenv("git")
secondbrain = os.getenv("secondbrain")
secondbrain_public = os.getenv("public_secondbrain")


# define paths
second_brain_path = str(secondbrain)  # "/tmp/second-brain-tmp"
public_folder_path_copy = str(secondbrain_public)
public_brain_image_path = os.path.join(public_folder_path_copy, "images")


regexp_md_images = "!\[\[(.*?)\]\](.*)\s"
h1 = "(?m)^#(?!#)(.*)"
h2 = "(?m)^#{2}(?!#)(.*)"
h3 = "(?m)^#{3}(?!#)(.*)"
h4 = "(?m)^#{4}(?!#)(.*)"
h5 = "(?m)^#{5}(?!#)(.*)"
h6 = "(?m)^#{6}(?!#)(.*)"


def find_hashtag(second_brain_path: str, copy_to_path: str) -> None:
    """find hashtag in my private SecondBrain and move it to public Brain.
    - replace h1 title (`# ..`) with frontmatter as YAML title
    - rename file names to lower-case
    """
    for root, dirs, files in os.walk(second_brain_path):
        for file in files:
            if file.endswith(".md"):
                file_path = os.path.join(root, file)
                with open(file_path, "r") as f:
                    for line_number, line in reversed(list(enumerate(f, 1))):
                        if "#publish" in line:
                            # destination should be lower-case (spaces will be handled by hugo with `urlize`)
                            file_name_lower = os.path.basename(file_path).lower()

                            # print(f"publish: {file_path}, ln: {line_number}")
                            # copy that file to the publish notes directory
                            shutil.copy(
                                file_path, os.path.join(copy_to_path, file_name_lower)
                            )
                            # get last modified date file_path
                            last_modified = datetime.utcfromtimestamp(
                                os.path.getmtime(file_path)
                            ).strftime("%Y-%m-%d %H:%M:%S")

                            # add h1 as title frontmatter
                            add_h1_as_title_frontmatter(
                                os.path.join(copy_to_path, file), last_modified
                            )
                            break


def add_h1_as_title_frontmatter(file_path: str, last_modified: str):
    print(f"start converting {file_path}")
    with open(file_path, "r") as f:
        content = pandoc.read(f.read(), format="markdown")

        headers = []
        for elt in pandoc.iter(content):
            if isinstance(elt, Header):
                if (
                    elt[0] == 1
                ):  # this is header 1, remove this if statement if you want all headers.
                    header = pandoc.write(elt[2]).strip()
                    headers.append(header)

                    # remove h1 (1 #) from content
                    content, lambda elt: elt != elt[2]

        # read line by line and search for h1
        # delete this line when found
        with open(file_path, "r") as f:
            lines = f.readlines()
            for line in lines:
                if re.search(h1, line):
                    # print(f"found h1 in {file_path}. removing...liine: {line}")
                    lines.remove(line)
                    break
        # write back to file
        with open(file_path, "w") as f:
            f.writelines(lines)

        # read file with frontmatter
        with open(file_path, "r") as f:
            frontmatter_post = frontmatter.load(f)
            # add h1 header to `title` to frontmatter
            if len(headers) > 0:
                frontmatter_post["title"] = headers[0]
            # add last modified date
            if len(last_modified):
                frontmatter_post["lastmod"] = last_modified
            # overwrite current file with added title
            with open(file_path, "wb") as f:
                frontmatter.dump(frontmatter_post, f)


def find_image_and_copy(image_name: str, root_path: str, public_brain_image_path: str):

    text_files = glob.glob(root_path + "/**/" + image_name, recursive=True)
    for file in text_files:
        shutil.copy(file, public_brain_image_path)
        # print(f"image `{file}` copied to {public_brain_image_path}")


def list_images_from_markdown(file_path: str):
    # search for images in markdown file
    file_content = open(file_path, "r").read()
    images = re.findall(regexp_md_images, file_content)
    if images:
        # print(f"-found images in {file_path}")
        for image in images:
            image_name = image[0]
            if image_name:
                # find image recursively in folder and copy to public image folder
                # print(f"--image: {image_name}")
                find_image_and_copy(
                    image_name, second_brain_path, public_folder_path_copy
                )

    # print(f"image: {file_path}, ln: {line}")
    pass


if __name__ == "__main__":
    find_hashtag(second_brain_path, public_folder_path_copy)
    # loop through public files and add referenced images, fix h1 headers and ..
    for root, dirs, files in os.walk(public_folder_path_copy):
        for file in files:
            if file.endswith(".md"):
                file_path = os.path.join(root, file)
                list_images_from_markdown(file_path)
                # print(f"converted: {file_path}")
