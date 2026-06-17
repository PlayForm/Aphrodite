"""aphrodite — marker subpackage: encoding, compression, preview, parsing."""

from .classify import _classify_content
from .compress import _compress_via_proxy, _get_conn, _put_conn
from .marker import _ccr_marker, _is_valid_ccr_hash
from .parse import _parse_ccr_markers, _parse_errors
from .preview import _make_ccr_preview

__all__ = [
    "_ccr_marker",
    "_classify_content",
    "_compress_via_proxy",
    "_get_conn",
    "_is_valid_ccr_hash",
    "_make_ccr_preview",
    "_parse_ccr_markers",
    "_parse_errors",
    "_put_conn",
]
