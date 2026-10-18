"""Scope fixture preview controls to their native toolbar and current file."""
import time
from body_navigation import APP, labels, rectangle, visible_owned

FILES = frozenset(('ocr_test.png', 'preview.txt', 'text_only.pdf', 'scanned.pdf'))
TOOLBAR = ('Back', 'Edit Attachment Attributes', 'Close')
PDF_IDS = frozenset(APP + ':id/' + name for name in ('pdfTitle', 'pdfPageImage', 'pdfCloseButton'))


def public_name(name):
    if name not in FILES:
        raise ValueError('An exact public fixture filename is required')


def toolbar_groups(nodes):
    groups = []
    for node in nodes:
        if node.get('package') != APP or node.get('class') != 'android.view.View':
            continue
        children = [child for child in node if child.tag == 'node']
        if all(any(label in labels(child) for child in children) for label in TOOLBAR):
            groups.append((node, children))
    return groups


def web_preview_close_candidates(nodes, name):
    public_name(name)
    groups = [(group, children) for group, children in toolbar_groups(nodes)
              if any(name in labels(child) for child in children)]
    if len(groups) != 1:
        return []
    group, children = groups[0]
    area = rectangle(group)
    if not visible_owned(group) or area is None:
        return []
    matched = {}
    for label in (*TOOLBAR, name):
        candidates = [child for child in children if label in labels(child)]
        if len(candidates) != 1:
            return []
        child = candidates[0]
        box = rectangle(child)
        if (not visible_owned(child) or box is None
                or not (area[0] <= box[0] < box[2] <= area[2]
                        and area[1] <= box[1] < box[3] <= area[3])):
            return []
        button = label in TOOLBAR
        if (child.get('class') != ('android.widget.Button' if button else 'android.widget.TextView')
                or child.get('clickable') != ('true' if button else 'false')):
            return []
        matched[label] = child
    return [matched['Close']]


def native_pdf_close_candidates(nodes, name):
    public_name(name)
    titles = [node for node in nodes if node.get('view-id') == APP + ':id/pdfTitle']
    closes = [node for node in nodes if node.get('view-id') == APP + ':id/pdfCloseButton']
    if len(titles) != 1 or len(closes) != 1:
        return []
    title, close = titles[0], closes[0]
    if (title.get('text') != name or not visible_owned(title) or not visible_owned(close)
            or rectangle(title) is None or rectangle(close) is None
            or close.get('class') != 'android.widget.ImageButton' or close.get('clickable') != 'true'):
        return []
    return closes


def native_pdf_present(nodes):
    return any(node.get('package') == APP and node.get('view-id') in PDF_IDS for node in nodes)


def preview_controls_present(nodes):
    # Presence is deliberately more conservative than actionability. A hidden,
    # disabled or wrong-file toolbar cannot make a close look completed.
    return bool(toolbar_groups(nodes)) or native_pdf_present(nodes)


def close_preview(journey, name, deadline, attachments_ready, save_receipt, *, clock=time.monotonic):
    """Close each real layer once; require settled layer absence, not background files."""
    public_name(name)
    closed = set()
    actions = []
    observations = 0
    first = None

    def candidates():
        if native_pdf_present(journey.all):
            return native_pdf_close_candidates(journey.nodes, name)
        return web_preview_close_candidates(journey.nodes, name)

    while journey.remaining(deadline) > 0:
        row = journey.snapshot(deadline)
        if row.get('rootPackage') != APP or row.get('allowedRoot') is not True:
            observations = 0
            first = None
            continue
        kind = 'native-pdf' if native_pdf_present(journey.all) else 'web-preview'
        if kind not in closed and candidates():
            target = journey.unique(candidates, deadline)
            kind = 'native-pdf' if target.get('view-id') == APP + ':id/pdfCloseButton' else 'web-preview'
            if kind not in closed:
                receipt = journey.observer.tap(target, journey.remaining(deadline))
                actions.append({'layer': kind, 'receipt': receipt})
                closed.add(kind)
            observations = 0
            first = None
            continue
        if closed and not preview_controls_present(journey.all) and attachments_ready():
            journey.focused(APP)
            now = clock()
            if first is None:
                first = now
            observations += 1
            if observations >= 3 and now - first >= .350:
                row = journey.snapshot(deadline, True)
                if (row.get('rootPackage') == APP and row.get('allowedRoot') is True
                        and not preview_controls_present(journey.all) and attachments_ready()):
                    save_receipt({'fileName': name, 'closedLayers': sorted(closed), 'actions': actions,
                                  'previewControlsAbsent': True, 'attachmentsRestored': True,
                                  'stableCompletionObservations': observations,
                                  'stableCompletionElapsedMs': (now - first) * 1000,
                                  'nativeScreenshot': row['screenshot']})
                    return row
        else:
            observations = 0
            first = None
    raise TimeoutError('Original preview close budget exhausted')
