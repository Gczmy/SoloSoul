"""Verify installed npm package records and versions against the product lockfile."""
from pathlib import Path, PurePosixPath

from prepare_avd import ordinary
from runtime_inputs import decode, digest


def installed_path(root,name):
    if not isinstance(name,str) or not name.startswith('node_modules/') or '\\' in name or ':' in name:
        raise ValueError('Locked node_modules package name required')
    parts=name.split('/')
    if any(p in ('','.', '..') for p in parts):raise ValueError('Locked package traversal refused')
    return ordinary(Path(root)/PurePosixPath(*parts[1:])/'package.json')


def verify(project,dependencies):
    project=ordinary(project,directory=True);dependencies=ordinary(dependencies,directory=True)
    paths=dict(project=project/'package.json',lock=project/'package-lock.json',installed=dependencies/'.package-lock.json')
    raw={n:ordinary(p).read_bytes() for n,p in paths.items()}
    package,lock,installed=(decode(raw[n]) for n in ('project','lock','installed'))
    if (type(lock.get('lockfileVersion')) is not int or lock['lockfileVersion']!=3
            or type(installed.get('lockfileVersion')) is not int or installed['lockfileVersion']!=3
            or not isinstance(lock.get('packages'),dict) or '' not in lock['packages']
            or not isinstance(installed.get('packages'),dict) or not installed['packages']):
        raise ValueError('Complete npm v3 product and installed lock records required')
    root=lock['packages']['']
    if any(package.get(n)!=lock.get(n) or package.get(n)!=root.get(n) for n in ('name','version')):
        raise ValueError('Product identity differs from lock')
    for group in ('dependencies','devDependencies','optionalDependencies'):
        if package.get(group,{})!=root.get(group,{}):raise ValueError('Product dependencies differ from lock')
    rows=installed['packages']
    required=set(package.get('dependencies',{}))|set(package.get('devDependencies',{}))
    required-=set(package.get('optionalDependencies',{}))
    if any('node_modules/'+name not in rows for name in required):
        raise ValueError('Required direct package is not installed')
    for name,row in rows.items():
        expected=lock['packages'].get(name)
        if not isinstance(row,dict) or not isinstance(expected,dict) or row.get('link') or expected.get('link'):
            raise ValueError('Unknown or linked installed package')
        if any(row.get(n)!=expected.get(n) for n in ('version','resolved','integrity')):
            raise ValueError('Installed package record differs from product lock')
        actual=decode(installed_path(dependencies,name).read_bytes())
        if not isinstance(row.get('version'),str) or not row['version'] or actual.get('version')!=row['version']:
            raise ValueError('Actual installed package version differs from lock')
    if any(p.read_bytes()!=raw[n] for n,p in paths.items()):raise ValueError('Dependency lock inputs changed')
    return dict(installedPackages=len(rows),requiredDirectPackages=len(required),
                installedRecordsMatchProductLock=True,actualPackageVersionsMatch=True,
                registryTarballIntegrityRecomputed=False,sourceSha256={n:digest(p) for n,p in paths.items()})
