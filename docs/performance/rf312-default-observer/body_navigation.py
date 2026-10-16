"""Locate the exact English public-fixture navigation group, never page titles."""
import re

APP = 'com.solosoul.app'
LABELS = frozenset(('Home', 'Objects', 'Tools', 'Settings'))


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
