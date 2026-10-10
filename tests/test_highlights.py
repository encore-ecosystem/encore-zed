#!/usr/bin/env python3
"""Check the actual Zed query's effective styles, not just capture presence."""

import argparse
from html.parser import HTMLParser
from pathlib import Path
import subprocess
import tempfile


class Highlights(HTMLParser):
    def __init__(self):
        super().__init__()
        self.in_line = False
        self.stack = []
        self.characters = []

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if tag == 'td' and attrs.get('class') == 'line':
            self.in_line = True
        elif tag == 'span' and self.in_line:
            self.stack.append(attrs.get('class', '').split())

    def handle_endtag(self, tag):
        if tag == 'td':
            self.in_line = False
        elif tag == 'span' and self.in_line:
            self.stack.pop()

    def handle_data(self, data):
        if self.in_line:
            self.characters.extend((character, tuple(tuple(item) for item in self.stack)) for character in data)


def main():
    root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser()
    parser.add_argument('--grammar', type=Path, default=root.parent / 'tree-sitter-encore')
    args = parser.parse_args()
    source = ('fn display(value: u32) -> str {\n'
              ' ret f"text value {value + 1_u32}; {read(value)}; {value.abs()}"\n}\n'
              'fn unicode(name: str) -> str { ret f"😀 Привет\nnext {name}!\\n" }\n')
    with tempfile.TemporaryDirectory(prefix='encore-zed-highlights-') as directory:
        path = Path(directory) / 'sample.enq'
        path.write_text(source)
        result = subprocess.run(['tree-sitter', 'highlight', '--html', '--css-classes',
            '--query-paths', str(root / 'languages/encore/highlights.scm'), '--', str(path)],
            cwd=args.grammar, capture_output=True, text=True, check=True, timeout=30)
    document = Highlights()
    document.feed(result.stdout)
    text = ''.join(character for character, _ in document.characters)
    assert text == source, (text, source)
    for needle, word, expected in [
        ('f"text', 'f', 'string'), ('text value', 'value', 'string'),
        ('{value +', 'value', 'variable'), ('value +', '+', 'operator'),
        ('1_u32', '1', 'number'), ('1_u32', '_u32', 'type builtin'),
        ('{read(value)}', 'read', 'function'), ('read(value)', 'value', 'variable'),
        ('{value.abs()}', 'abs', 'function'), ('next {name}', 'name', 'variable'),
        ('!\\n', '\\n', 'string'),
    ]:
        offset = text.index(needle) + needle.index(word)
        styles = document.characters[offset][1]
        assert styles and styles[-1] == tuple(expected.split()), (needle, word, styles)
        if expected in ('variable', 'function', 'operator', 'number'):
            assert not any('string' in style for style in styles), (needle, styles)
    print('Zed query: 11 effective styles, Unicode, multiline strings and no string overlay; ok')


if __name__ == '__main__':
    main()
