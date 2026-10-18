"""Describe existing native control intervals after canonical UI verification.

Elapsed method time can include application waits and input dispatch. It is not
exclusive observer CPU cost, application latency, or evidence of drawn frames.
"""
import hashlib
import json
from pathlib import Path
import re

MAX_BYTES = 8 * 1024 * 1024
MAX_REQUESTS = 512
ACTIONS = frozenset(('snapshot', 'inspect-target', 'tap', 'type-public',
                     'pick-public-file', 'scroll', 'back'))


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def raw_file(path):
    path = Path(path)
    if not path.is_file() or path.is_symlink() or path.is_junction():
        raise ValueError('Ordinary timing input required')
    with path.open('rb') as stream:
        raw = stream.read(MAX_BYTES + 1)
    if not 0 < len(raw) <= MAX_BYTES:
        raise ValueError('Timing input exceeds bound')
    return raw


def decode(raw):
    def unique(pairs):
        value = {}
        for key, item in pairs:
            if key in value:
                raise ValueError('Duplicate timing input field')
            value[key] = item
        return value
    def constant(_):
        raise ValueError('Non-finite timing input')
    return json.loads(raw, object_pairs_hook=unique, parse_constant=constant)


def files(folder):
    folder = Path(folder)
    if not folder.is_dir() or folder.is_symlink() or folder.is_junction():
        raise ValueError('Ordinary observer directory required')
    found = {}
    for prefix in ('request', 'response'):
        names = sorted(path.name for path in folder.glob(prefix + '-*.json'))
        if not 0 < len(names) <= MAX_REQUESTS:
            raise ValueError('Bounded complete observer requests required')
        if names != [f'{prefix}-{i:03d}.json' for i in range(1, len(names) + 1)]:
            raise ValueError('Contiguous original protocol files required')
        found[prefix] = names
    if len(found['request']) != len(found['response']):
        raise ValueError('Missing native request or response')
    return found


def fingerprint(folder, phase_file):
    names = files(folder)
    value = {name: digest(raw_file(Path(folder) / name))
             for group in names.values() for name in group}
    value['phaseFile'] = digest(raw_file(phase_file))
    return value


def statistics(values):
    ordered = sorted(values)
    if not ordered:
        raise ValueError('Non-empty interval population required')
    n = len(ordered)
    numerator = ordered[n // 2] if n % 2 else ordered[n // 2 - 1] + ordered[n // 2]
    return dict(n=n, sumNs=sum(ordered), minimumNs=ordered[0], maximumNs=ordered[-1],
                medianNumeratorNs=numerator, medianDenominator=1 if n % 2 else 2,
                nearestRankP95Ns=ordered[(95 * n + 99) // 100 - 1])


def collect(folder, phase_file, expected_phases, frozen):
    folder = Path(folder)
    names = files(folder)
    categories, rows, pairs, by_image = {}, [], [], {}
    previous_end, previous_reply = None, None
    for ordinal, (request_name, response_name) in enumerate(zip(names['request'], names['response']), 1):
        raw_request, raw_response = raw_file(folder / request_name), raw_file(folder / response_name)
        if digest(raw_request) != frozen[request_name] or digest(raw_response) != frozen[response_name]:
            raise ValueError('Timing input changed after canonical verification')
        request, response = decode(raw_request), decode(raw_response)
        if not isinstance(request, dict) or not isinstance(response, dict):
            raise ValueError('Original request and response objects required')
        action = request.get('action')
        if action not in ACTIONS or response.get('action') != action or response.get('success') is not True:
            raise ValueError('Complete successful original action receipt required')
        start, end = response.get('nativeStartedAtNs'), response.get('nativeFinishedAtNs')
        if (type(start) is not int or type(end) is not int or not 0 < start <= end < 2**63
                or previous_end is not None and start < previous_end):
            raise ValueError('Invalid or overlapping native control clocks')
        image = response.get('screenshot')
        if action == 'snapshot' and type(request.get('screenshot', False)) is not bool:
            raise ValueError('Original snapshot boolean required')
        if image is not None and (action != 'snapshot' or request.get('screenshot') is not True
                                  or image != f'frame-{ordinal:03d}.png'):
            raise ValueError('Native screenshot protocol identity differs')
        if action == 'snapshot' and request.get('screenshot') is True and image is None:
            raise ValueError('Requested screenshot receipt missing')
        kind = ('snapshot-with-png' if image else 'snapshot-metadata') if action == 'snapshot' else action
        duration = end - start
        categories.setdefault(kind, []).append(duration)
        row = dict(ordinal=ordinal, action=action, kind=kind, nativeStartedAtNs=start,
                   nativeFinishedAtNs=end, durationNs=duration,
                   requestSha256=digest(raw_request), responseSha256=digest(raw_response))
        if image:
            by_image[image] = row
            if (rows and rows[-1]['kind'] == 'snapshot-metadata'
                    and isinstance(response.get('xml'), str)
                    and previous_reply.get('xml') == response['xml']
                    and previous_reply.get('rootPackage') == response.get('rootPackage')):
                pairs.append(dict(metadataOrdinal=ordinal - 1, imageOrdinal=ordinal,
                                  incrementNs=duration - rows[-1]['durationNs']))
        rows.append(row)
        previous_end, previous_reply = end, response
    raw_phases = raw_file(phase_file)
    if digest(raw_phases) != frozen['phaseFile']:
        raise ValueError('Phase input changed after canonical verification')
    phases = decode(raw_phases)
    if (not isinstance(phases, list) or not all(isinstance(p, dict) for p in phases)
            or [p.get('name') for p in phases] != list(expected_phases)):
        raise ValueError('Original complete phase order required')
    windows, last_ordinal = [], 0
    for phase in phases:
        image = phase.get('nativeScreenshot')
        if not isinstance(image, str) or not re.fullmatch(r'frame-\d{3}\.png', image) or image not in by_image:
            raise ValueError('Original phase screenshot response required')
        row = by_image[image]
        if row['ordinal'] <= last_ordinal:
            raise ValueError('Original phase observations must advance')
        last_ordinal = row['ordinal']
        windows.append(dict(name=phase['name'], responseOrdinal=row['ordinal'],
                            nativeStartedAtNs=row['nativeStartedAtNs'], nativeFinishedAtNs=row['nativeFinishedAtNs']))
    if fingerprint(folder, phase_file) != frozen:
        raise ValueError('Original timing files changed during analysis')
    span = rows[-1]['nativeFinishedAtNs'] - rows[0]['nativeStartedAtNs']
    total = sum(row['durationNs'] for row in rows)
    if total > span:
        raise ValueError('Non-overlapping elapsed intervals required')
    return dict(scope='native-control-method-elapsed-intervals', requests=len(rows),
                nativeChannelSpanNs=span, nativeOperationDurationSumNs=total,
                nonOverlappingNativeIntervals=True,
                operationPopulations={k: statistics(v) for k, v in sorted(categories.items())},
                consecutiveMatchingXmlPairs=pairs, phaseScreenshotObservationWindows=windows,
                inputSha256=frozen, rows=rows, canonicalUiValidationPerformedByThisLibrary=False,
                matchedPairsHaveNoRandomizedOrderControl=True, exclusiveObserverOverheadAccepted=False,
                applicationOnlyLatencyAccepted=False, firstInteractiveTimeProven=False,
                nativeFramesAccepted=False, performanceDistributionAccepted=False, fullRF312Accepted=False)
