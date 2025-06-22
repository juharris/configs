"""
Deletes duplicate images in the Dropbox Camera Uploads folder based on the file name.
Keeps the latest version based on a suffix of the file name.

Example: 2025-06-19 09.45.31.jpg, 2025-06-19 09.45.31-1.jpg, 2025-06-19 09.45.31-2.jpg
The script will delete 2025-06-19 09.45.31.jpg and 2025-06-19 09.45.31-1.jpg, but keep 2025-06-19 09.45.31-2.jpg
"""

import argparse
import os
import re
import sys
from collections import defaultdict
from pathlib import Path

from tqdm import tqdm


def parse_filename(filename: str) -> tuple[str, int]:
    """
    Parse a filename to extract the base name and suffix number.
    
    Args:
        filename (str): The filename to parse
        
    Returns:
        tuple: (base_name, suffix_num) where suffix_num is 0 if no suffix
    """
    match = re.match(r'^(.+)-(\d+).([^.]+)$', filename)
    if match:
        # Has numeric suffix
        extension = f'.{match.group(3)}'
        base_name = match.group(1) + extension
        suffix_num = int(match.group(2))
    else:
        # No numeric suffix
        base_name = filename
        suffix_num = 0
    
    return base_name, suffix_num


def count_files(folder: str) -> int:
    return len([f for f in Path(folder).rglob('*') if f.is_file()])


def clean_dropbox_pics(folder_path, dry_run=False):
    """
    Clean duplicate images in the specified folder.
    
    Args:
        folder_path (str): Path to the folder to clean
        dry_run (bool): If True, show what would be deleted without actually deleting
    """
    folder = Path(folder_path)
    
    if not folder.exists() or not folder.is_dir():
        print(f"Directory {folder} does not exist.", file=sys.stderr)
        sys.exit(1)
    
    num_files_before = count_files(folder)
    print(f"Number of files before cleanup: {num_files_before}")
    
    # Group files by base name
    file_groups: defaultdict[str, list[tuple[int, str]]] = defaultdict(list)

    for file_path in tqdm(folder.rglob('*'), desc="Grouping files...", unit=" file"):
        if not file_path.is_file():
            continue
        basename = file_path.name
        base_name, suffix_num = parse_filename(basename)
        
        file_groups[base_name].append((suffix_num, file_path))
    
    # Filter groups that have duplicates
    duplicate_groups = {base_name: files for base_name, files in file_groups.items() if len(files) > 1}
    
    if len(duplicate_groups) == 0:
        print("No duplicate files found.")
        return
    
    print(f"Found {len(duplicate_groups)} groups with duplicates.")
    
    for base_name, files in tqdm(duplicate_groups.items(), desc="Processing duplicates...", unit=" group"):
        # Find the file with the highest suffix number
        max_suffix = -1
        keep_file = None
        
        for suffix_num, file_path in files:
            if suffix_num > max_suffix:
                max_suffix = suffix_num
                keep_file = file_path
        
        for _, file_path in files:
            if file_path == keep_file:
                continue
            if dry_run:
                print(f"Would delete: {file_path} (keeping {keep_file})")
            else:
                # print(f"Deleting: {file_path} (keeping {keep_file})")
                file_path.unlink()
    
    num_files_after = count_files(folder)
    print(f"Number of files after cleanup: {num_files_after}")


def main():
    """Main entry point."""
    parser = argparse.ArgumentParser(
        description="Delete duplicate images in Dropbox Camera Uploads folder"
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Show what would be deleted without actually deleting files"
    )
    parser.add_argument(
        "--folder",
        default=os.path.expanduser("~/Dropbox/Camera Uploads"),
        help="Folder to clean (default: ~/Dropbox/Camera Uploads)"
    )
    
    args = parser.parse_args()
    
    clean_dropbox_pics(args.folder, args.dry_run)


if __name__ == "__main__":
    main()
