"""Verify the ARM64 APK implements every app-native method declared in its DEX files."""
import argparse
import re
import subprocess
import tempfile
import xml.etree.ElementTree as ET
import zipfile
from pathlib import Path


def jni_name(value):
    result = ''
    for character in value:
        if character in 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789':
            result += character
        elif character in '/_;[':
            result += {'/': '_', '_': '_1', ';': '_2', '[': '_3'}[character]
        else:
            encoded = character.encode('utf-16-be')
            result += ''.join(f'_0{int.from_bytes(encoded[i:i+2], "big"):04x}' for i in range(0, len(encoded), 2))
    return result


def descriptor(name):
    if name.endswith('[]'):
        return '[' + descriptor(name[:-2])
    primitives = {'boolean': 'Z', 'byte': 'B', 'char': 'C', 'short': 'S', 'int': 'I', 'long': 'J', 'float': 'F', 'double': 'D', 'void': 'V'}
    return primitives.get(name, 'L' + name.replace('.', '/') + ';')


def required_methods(xml):
    methods = []
    for package in ET.fromstring(xml).iter('package'):
        name = package.attrib['name']
        if name != 'health.app' and not name.startswith('health.app.'):
            continue
        for kind in package.findall('class'):
            owner = (name + '.' + kind.attrib['name']).replace('.', '/')
            for method in kind.findall('method'):
                if method.get('native') != 'true':
                    continue
                short = 'Java_' + jni_name(owner) + '_' + jni_name(method.attrib['name'])
                parameters = ''.join(descriptor(p.attrib['type']) for p in method.findall('parameter'))
                methods.append((short, short + '__' + jni_name(parameters)))
    return methods


def validate_package(abis, methods, symbols):
    if abis != {'arm64-v8a'}:
        raise ValueError(f'APK must contain only ARM64 libraries; found {sorted(abis)}')
    if not methods:
        raise ValueError('No app-native declarations found in APK DEX files')
    missing = [short for short, long in methods if short not in symbols and long not in symbols]
    if missing:
        raise ValueError('ARM64 app library is missing JNI exports: ' + ', '.join(sorted(missing)))


def inspect_package(apk, dexdump):
    methods = []
    with zipfile.ZipFile(apk) as archive, tempfile.TemporaryDirectory() as temp:
        abis = {name.split('/')[1] for name in archive.namelist() if name.startswith('lib/') and name.endswith('.so')}
        library = Path(temp) / 'libHealthApp.so'
        library.write_bytes(archive.read('lib/arm64-v8a/libHealthApp.so'))
        output = subprocess.check_output(['nm', '-D', '--defined-only', str(library)], text=True)
        symbols = {line.split()[-1] for line in output.splitlines() if line.split()}
        for name in archive.namelist():
            if re.fullmatch(r'classes\d*\.dex', name):
                dex = Path(temp) / name
                dex.write_bytes(archive.read(name))
                methods.extend(required_methods(subprocess.check_output([str(dexdump), '-l', 'xml', str(dex)], text=True)))
    validate_package(abis, methods, symbols)
    print(f'ARM64 APK verification passed: all {len(methods)} app-native methods are implemented')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('apk', type=Path)
    parser.add_argument('--dexdump', type=Path, required=True)
    args = parser.parse_args()
    inspect_package(args.apk, args.dexdump)
