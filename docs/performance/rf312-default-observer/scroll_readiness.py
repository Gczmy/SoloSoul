"""Defer read-only scroll readiness; never retry a dispatched native gesture."""


def scroll_if_ready(journey, node, direction, deadline):
    journey.focused(node.get('package'))
    probe = journey.observer.inspect(node, 'scroll', journey.remaining(deadline))
    if not journey.observer.target_ready(node, probe, 'scroll'):
        return False
    receipt = journey.observer.scroll(node, direction, journey.remaining(deadline))
    journey.scrolls.append(receipt)
    return True
