import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import App from './App';
import { siteContent } from './content';

describe('qedit marketing site', () => {
  it('renders the primary product story and navigation anchors', () => {
    render(<App />);

    expect(screen.getByRole('heading', { level: 1 }).textContent).toMatch(
      /your files,\s*open fast/i,
    );
    expect(
      screen
        .getAllByRole('link', { name: 'Features' })[0]
        ?.getAttribute('href'),
    ).toBe('#features');
    expect(
      screen.getByRole('link', { name: 'Workflow' }).getAttribute('href'),
    ).toBe('#workflow');
    expect(
      screen
        .getAllByRole('link', { name: 'Download' })
        .find((link) => link.getAttribute('href') === siteContent.releaseUrl),
    ).toBeTruthy();
  });

  it('points platform downloads and support to centralized external links', () => {
    render(<App />);

    expect(
      screen.getByRole('heading', { name: /pick your platform/i }),
    ).toBeTruthy();
    expect(
      screen.getByRole('link', { name: /macos/i }).getAttribute('href'),
    ).toBe(siteContent.releaseUrl);
    expect(
      screen.getByRole('link', { name: /windows/i }).getAttribute('href'),
    ).toBe(siteContent.releaseUrl);
    expect(
      screen.getByRole('link', { name: /linux/i }).getAttribute('href'),
    ).toBe(siteContent.releaseUrl);
    expect(
      screen.getByRole('link', { name: /buy us a beer/i }).getAttribute('href'),
    ).toBe(siteContent.donationUrl);
  });
});
