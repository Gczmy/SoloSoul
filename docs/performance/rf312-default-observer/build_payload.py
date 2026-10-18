"""Read the immutable, materialized output of the production frontend build."""
from pathlib import Path
import re

from frontend_build import FIELDS, GENERATED_TRACKED, builder_sources, check_outputs, files, validate_files
from prepare_avd import ordinary
from runtime_inputs import decode, digest, hash_value


def load(path, expected_sha256):
    path=ordinary(path)
    if digest(path)!=hash_value(expected_sha256):raise ValueError('Declared frontend receipt changed')
    raw=path.read_bytes();result=decode(raw)
    if (result.get('passed') is not True or type(result.get('exitCode')) is not int or result['exitCode']!=0
            or result.get('frontendBuiltFromCopiedSources') is not True
            or result.get('originalInputsUnchanged') is not True
            or result.get('productInstalled') is not False
            or type(result.get('deviceCommandsExecuted')) is not int or result['deviceCommandsExecuted']!=0
            or type(result.get('targetInputsDispatched')) is not int or result['targetInputsDispatched']!=0):
        raise ValueError('Passing isolated production frontend receipt required')
    plan_path=ordinary(path.parent/'frontend-plan.json')
    if digest(plan_path)!=hash_value(result['planSha256']):raise ValueError('Bound frontend plan changed')
    plan=decode(plan_path.read_bytes())
    if (set(plan)!=FIELDS or type(plan['schemaVersion']) is not int or plan['schemaVersion']!=1
            or result['builderSources']!=builder_sources() or plan['builderSources']!=builder_sources()
            or not isinstance(result['sourceHead'],str) or not re.fullmatch('[a-f0-9]{40}',result['sourceHead'])
            or not isinstance(result['pluginHead'],str) or not re.fullmatch('[a-f0-9]{40}',result['pluginHead'])
            or plan['sourceHead']!=result['sourceHead'] or plan['pluginHead']!=result['pluginHead']):
        raise ValueError('Recorded frontend builder and source identity required')
    workspace=ordinary(path.parent/'workspace',directory=True)
    frontend=ordinary(path.parent/'frontend',directory=True)
    if (ordinary(result['sourceWorkspace'],directory=True)!=workspace
            or ordinary(result['frontendRoot'],directory=True)!=frontend):
        raise ValueError('Frontend materialized paths differ from recorded layout')
    for rows in (plan['sourceFiles'],plan['pluginFiles'],plan['nodeModules']['files'],result['generatedTrackedFiles']):
        if rows:validate_files(rows)
    if (not plan['sourceFiles'] or not plan['pluginFiles'] or not plan['nodeModules']['files']
            or any(not n.startswith('tauri/') for n in plan['sourceFiles'])
            or set(result['generatedTrackedFiles'])-GENERATED_TRACKED
            or not set(result['generatedTrackedFiles']).issubset(plan['sourceFiles'])
            or any(type(result[k]) is not int for k in ('sourceFiles','pluginFiles','dependencyFiles'))
            or result['sourceFiles']!=len(plan['sourceFiles'])
            or result['pluginFiles']!=len(plan['pluginFiles'])
            or result['dependencyFiles']!=len(plan['nodeModules']['files'])):
        raise ValueError('Complete original and derived frontend input bindings required')
    expected={**plan['sourceFiles'],**result['generatedTrackedFiles']}
    if files(workspace,expected)!=expected:raise ValueError('Materialized product source changed')
    if files(workspace/'SoloSoul_plugin_market')!=plan['pluginFiles']:
        raise ValueError('Materialized complete plugin source changed')
    outputs=check_outputs(workspace,frontend)
    if outputs!=result['outputs']:raise ValueError('Materialized frontend or resources changed')
    if path.read_bytes()!=raw:raise ValueError('Frontend receipt changed during validation')
    # The producer already froze its original checkout during the build. Later
    # documentation commits do not invalidate this separately hashed source copy.
    # No path under plan['workspace'] or plan['nodeModules']['root'] is read here.
    return dict(receipt=str(path),receiptSha256=expected_sha256,planSha256=result['planSha256'],
        workspace=str(workspace),frontend=str(frontend),sourceHead=result['sourceHead'],pluginHead=result['pluginHead'],
        sources=expected,plugins=plan['pluginFiles'],outputs=outputs,producerSources=result['builderSources'])


def recheck(payload):
    if load(payload['receipt'],payload['receiptSha256'])!=payload:
        raise ValueError('Frozen materialized frontend payload changed')
