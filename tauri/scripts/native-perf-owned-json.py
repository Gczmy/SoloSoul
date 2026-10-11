"""Read published Windows diagnostic JSON without excluding DELETE sharing.

This is a read-only verification helper, not an application API. Callers must
still validate the owned root, nonce, process identity and proof schema.
"""
import ctypes
from ctypes import wintypes as w
from contextlib import contextmanager
import json
import os
from pathlib import Path


class FileInfo(ctypes.Structure):
    _fields_ = [
        ("attributes", w.DWORD), ("created", w.FILETIME),
        ("accessed", w.FILETIME), ("written", w.FILETIME),
        ("volume", w.DWORD), ("sizeHigh", w.DWORD), ("sizeLow", w.DWORD),
        ("links", w.DWORD), ("indexHigh", w.DWORD), ("indexLow", w.DWORD),
    ]


def _api():
    if os.name != "nt":
        raise RuntimeError("Owned JSON shared-handle verification requires Windows")
    api = ctypes.WinDLL("kernel32", use_last_error=True)
    api.CreateFileW.argtypes = [w.LPCWSTR, w.DWORD, w.DWORD, w.LPVOID, w.DWORD, w.DWORD, w.HANDLE]
    api.CreateFileW.restype = w.HANDLE
    api.CloseHandle.argtypes = [w.HANDLE]
    api.CloseHandle.restype = w.BOOL
    api.GetFileType.argtypes = [w.HANDLE]
    api.GetFileType.restype = w.DWORD
    api.GetFileInformationByHandle.argtypes = [w.HANDLE, ctypes.POINTER(FileInfo)]
    api.GetFileInformationByHandle.restype = w.BOOL
    api.GetFinalPathNameByHandleW.argtypes = [w.HANDLE, w.LPWSTR, w.DWORD, w.DWORD]
    api.GetFinalPathNameByHandleW.restype = w.DWORD
    api.ReadFile.argtypes = [w.HANDLE, w.LPVOID, w.DWORD, ctypes.POINTER(w.DWORD), w.LPVOID]
    api.ReadFile.restype = w.BOOL
    return api


def _require(ok):
    if not ok:
        raise ctypes.WinError(ctypes.get_last_error())


def _normal(path):
    value = os.fspath(path)
    if value.startswith("\\\\?\\"):
        value = value[4:]
    return os.path.normcase(os.path.abspath(value))


@contextmanager
def _open(api, path):
    # GENERIC_READ, FILE_SHARE_READ|WRITE|DELETE, OPEN_EXISTING,
    # FILE_FLAG_OPEN_REPARSE_POINT. No creation, writes, ACL changes or retry.
    handle = api.CreateFileW(str(path), 0x80000000, 7, None, 3, 0x00200000, None)
    if handle == ctypes.c_void_p(-1).value:
        raise ctypes.WinError(ctypes.get_last_error())
    try:
        yield handle
    finally:
        _require(api.CloseHandle(handle))


def _info(api, handle, expected, maximum):
    info = FileInfo()
    _require(api.GetFileInformationByHandle(handle, ctypes.byref(info)))
    if api.GetFileType(handle) != 1 or info.attributes & (0x10 | 0x400):
        raise ValueError("Diagnostic marker must be an ordinary non-reparse disk file")
    size = (info.sizeHigh << 32) | info.sizeLow
    if size > maximum:
        raise ValueError("Diagnostic marker exceeds its byte limit")
    resolved = ctypes.create_unicode_buffer(32768)
    count = api.GetFinalPathNameByHandleW(handle, resolved, len(resolved), 0)
    _require(count)
    if count >= len(resolved) or _normal(resolved.value) != _normal(expected):
        raise ValueError("Diagnostic marker resolved outside its requested path")
    return info, size


def _identity(info):
    return info.volume, info.indexHigh, info.indexLow


def read_owned_json(path, maximum=262144):
    """Read one existing bounded marker; preserve errors and reject changed files."""
    path = Path(path)
    if not path.is_absolute() or not path.drive or path == Path(path.anchor):
        raise ValueError("Diagnostic marker requires an absolute non-root path")
    if isinstance(maximum, bool) or not isinstance(maximum, int) or not 1 <= maximum <= 4194304:
        raise ValueError("Diagnostic marker byte limit is invalid")
    api = _api()
    with _open(api, path) as handle:
        before, size = _info(api, handle, path, maximum)
        data = bytearray()
        while True:
            # Read one extra byte to reject growth even when the first size was valid.
            length = min(65536, maximum + 1 - len(data))
            buffer = ctypes.create_string_buffer(length)
            read = w.DWORD()
            _require(api.ReadFile(handle, buffer, length, ctypes.byref(read), None))
            if not read.value:
                break
            data.extend(buffer.raw[:read.value])
            if len(data) > maximum:
                raise ValueError("Diagnostic marker grew beyond its byte limit")
        after, after_size = _info(api, handle, path, maximum)
        if _identity(before) != _identity(after) or len(data) != size or size != after_size:
            raise ValueError("Diagnostic marker identity or size changed during reading")
        if (before.written.dwHighDateTime, before.written.dwLowDateTime) != (
            after.written.dwHighDateTime, after.written.dwLowDateTime
        ):
            raise ValueError("Diagnostic marker changed during reading")
        # A held handle can survive replacement of its name. Verify that the
        # requested name still identifies this file, without reading another file.
        with _open(api, path) as current:
            current_info, current_size = _info(api, current, path, maximum)
            if _identity(current_info) != _identity(after) or current_size != size:
                raise ValueError("Diagnostic marker path was replaced during reading")
    return json.loads(data.decode("utf-8-sig"))
