"""Locate the exact English public-fixture navigation group, never page titles."""
import re

APP = 'com.solosoul.app'
LABELS = frozenset(('Home', 'Objects', 'Tools', 'Settings'))
TOOLS_BODY = ('Organize, connect, explore', 'Your everyday tools, together in one place.')
BACKUP_REMINDER = ('It has been a while since your last backup. Please go to Settings > '
                   'Backup & Restore to create one.')


def labels(node):
    return {value for value in (node.get('text', ''), node.get('content-desc', '')) if value}


def rectangle(node):
    match = re.fullmatch(r'\[(\d+),(\d+)\]\[(\d+),(\d+)\]', node.get('bounds', ''))
    if not match:
        return None
    left, top, right, bottom = map(int, match.groups())
    return (left, top, right, bottom) if left < right and top < bottom else None


def visible_owned(node):
    return (node.get('package') == APP and node.get('visible-to-user') == 'true'
            and node.get('enabled') == 'true')


def main_navigation_top(nodes):
    """Return the owned group's top, or None until every native condition matches."""
    groups = [node for node in nodes if 'Main navigation' in labels(node)]
    if len(groups) != 1:
        return None
    group = groups[0]
    area = rectangle(group)
    if (not visible_owned(group) or group.get('class') != 'android.view.View'
            or group.get('clickable') != 'false' or area is None):
        return None
    targets = {label: [] for label in LABELS}
    for node in group.iter('node'):
        names = labels(node) & LABELS
        if not names or node.get('clickable') != 'true':
            continue
        box = rectangle(node)
        if (len(names) != 1 or not visible_owned(node) or box is None
                or not (area[0] <= box[0] < box[2] <= area[2]
                        and area[1] <= box[1] < box[3] <= area[3])):
            return None
        targets[next(iter(names))].append(node)
    if any(len(items) != 1 for items in targets.values()):
        return None
    return area[1]


def tool_card_candidates(nodes, tool):
    """Match a frozen public Tools card only above the verified navigation."""
    if tool not in ('OCR', 'Search'):
        raise ValueError('Known public tool card required')
    limit = main_navigation_top(nodes)
    if limit is None:
        return []
    result = []
    for node in nodes:
        area = rectangle(node)
        if (visible_owned(node) and node.get('class') == 'android.widget.Button'
                and node.get('clickable') == 'true' and node.get('text') == tool
                and area is not None and (area[1] + area[3]) / 2 < limit):
            if not any(node is old for old in result):
                result.append(node)
    return result


def tools_layout_ready(nodes):
    """Wait for the owned Tools body and the known inline reminder to leave."""
    limit = main_navigation_top(nodes)
    if limit is None:
        return False
    group = next(node for node in nodes if 'Main navigation' in labels(node))
    current = [node for node in group.iter('node')
               if 'Tools, current page' in labels(node) and node.get('clickable') == 'true']
    if len(current) != 1:
        return False
    for text in TOOLS_BODY:
        found = [node for node in nodes if text in labels(node) and visible_owned(node)
                 and rectangle(node) is not None and rectangle(node)[3] < limit]
        if len(found) != 1:
            return False
    # A disabled reminder can still occupy layout space. Inspect the original
    # unfiltered owned hierarchy, not only the actionable/visible node list.
    return not any(node.get('package') == APP and node.get('visible-to-user') == 'true'
                   and any(' '.join(value.split()).startswith(BACKUP_REMINDER) for value in labels(node))
                   for node in nodes)


def open_tool_card(journey, tool, deadline):
    """Reuse original native scroll/readiness and dispatch one exact tap."""
    if tool not in ('OCR', 'Search'):
        raise ValueError('Known public tool card required')
    journey.wait(lambda: tools_layout_ready(journey.all), deadline)
    target = journey.unique(lambda: tool_card_candidates(journey.nodes, tool), deadline, allow_scroll=True)
    return journey.observer.tap(target, journey.remaining(deadline))
