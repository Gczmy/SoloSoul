"""Bounded inventory for a used forensic AVD; default fresh-AVD helper unchanged."""
from rf312_ipc_library import ordinary
from frontend_build import entry

BLOCK=4*1024*1024


def digest_file(path):
    # Canonical reader streams via hashlib.file_digest and freezes path and
    # handle attributes separately, including Windows filename-derived modes.
    return entry(path)['sha256']


def inventory(path):
    root=ordinary(path,directory=True)
    files,folders={},[]
    for item in sorted(root.rglob('*')):
        directory=item.is_dir();ordinary(item,directory=directory)
        if directory:folders.append(item.relative_to(root).as_posix())
        else:files[item.relative_to(root).as_posix()]=digest_file(item)
    return dict(files=files,directories=folders)
