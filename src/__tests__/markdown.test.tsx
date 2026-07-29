import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { MarkdownPreview } from '../lib/markdown';

describe('MarkdownPreview', () => {
  it('renders readable markdown blocks without interpreting raw HTML', () => {
    render(
      <MarkdownPreview
        content={
          '# Notes\n\n**bold** and `code`\n\n```ts\nconst answer = 42;\n```\n\n<script>alert(1)</script>'
        }
      />,
    );

    expect(screen.getByRole('heading', { name: 'Notes' })).toBeTruthy();
    expect(screen.getByText('bold')).toBeTruthy();
    expect(screen.getByText('const answer = 42;')).toBeTruthy();
    expect(screen.getByText('<script>alert(1)</script>')).toBeTruthy();
    expect(document.querySelector('script')).toBeNull();
  });

  it('only turns safe links into anchors', () => {
    render(
      <MarkdownPreview
        content={'[safe](https://example.com) [unsafe](javascript:alert(1))'}
      />,
    );

    expect(screen.getByRole('link', { name: 'safe' }).getAttribute('rel')).toBe(
      'noreferrer noopener',
    );
    expect(screen.queryByRole('link', { name: 'unsafe' })).toBeNull();
    expect(screen.getByText('unsafe')).toBeTruthy();
  });
});
