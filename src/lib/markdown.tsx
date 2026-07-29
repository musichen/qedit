import type { ReactNode } from 'react';

function safeLink(url: string): string | null {
  const trimmed = url.trim();

  return /^(https?:|mailto:)/i.test(trimmed) ? trimmed : null;
}

function inlineMarkdown(text: string, keyPrefix: string): ReactNode[] {
  const nodes: ReactNode[] = [];
  const tokenPattern =
    /(\[([^\]]+)\]\(([^)]+)\)|`([^`]+)`|\*\*([^*]+)\*\*|\*([^*]+)\*)/g;
  let lastIndex = 0;
  let match: RegExpExecArray | null;
  let index = 0;

  while ((match = tokenPattern.exec(text)) !== null) {
    if (match.index > lastIndex) nodes.push(text.slice(lastIndex, match.index));

    const key = `${keyPrefix}-${index++}`;
    if (match[2] && match[3]) {
      const url = safeLink(match[3]);
      nodes.push(
        url ? (
          <a
            key={key}
            href={url}
            target={/^https?:/i.test(url) ? '_blank' : undefined}
            rel={/^https?:/i.test(url) ? 'noreferrer noopener' : undefined}
            className="text-primary underline underline-offset-2 hover:text-primary/80"
          >
            {match[2]}
          </a>
        ) : (
          <span key={key} title="Only http(s) and mailto links are enabled">
            {match[2]}
          </span>
        ),
      );
    } else if (match[4]) {
      nodes.push(
        <code key={key} className="rounded bg-muted px-1 py-0.5 text-[0.9em]">
          {match[4]}
        </code>,
      );
    } else if (match[5]) {
      nodes.push(<strong key={key}>{match[5]}</strong>);
    } else if (match[6]) {
      nodes.push(<em key={key}>{match[6]}</em>);
    }

    lastIndex = tokenPattern.lastIndex;
  }

  if (lastIndex < text.length) nodes.push(text.slice(lastIndex));

  return nodes;
}

function isBlockStart(line: string): boolean {
  return (
    /^#{1,6}\s+/.test(line) ||
    /^```/.test(line) ||
    /^>\s?/.test(line) ||
    /^[-*+]\s+/.test(line) ||
    /^\d+\.\s+/.test(line) ||
    /^---+$/.test(line)
  );
}

function headingNode(
  level: number,
  content: ReactNode,
  key: string,
): ReactNode {
  const className =
    level === 1
      ? 'text-2xl font-semibold'
      : level === 2
        ? 'text-xl font-semibold'
        : 'text-lg font-semibold';

  switch (level) {
    case 1:
      return (
        <h1 key={key} className={className}>
          {content}
        </h1>
      );
    case 2:
      return (
        <h2 key={key} className={className}>
          {content}
        </h2>
      );
    case 3:
      return (
        <h3 key={key} className={className}>
          {content}
        </h3>
      );
    case 4:
      return (
        <h4 key={key} className={className}>
          {content}
        </h4>
      );
    case 5:
      return (
        <h5 key={key} className={className}>
          {content}
        </h5>
      );
    default:
      return (
        <h6 key={key} className={className}>
          {content}
        </h6>
      );
  }
}

/**
 * A deliberately small Markdown renderer for local notes. It produces React
 * nodes instead of HTML, so file contents cannot inject markup or scripts.
 */
export function MarkdownPreview({ content }: { content: string }) {
  const lines = content.replaceAll('\r\n', '\n').split('\n');
  const blocks: ReactNode[] = [];
  let index = 0;
  let blockIndex = 0;

  while (index < lines.length) {
    const line = lines[index] ?? '';
    if (!line.trim()) {
      index += 1;
      continue;
    }

    const fence = line.match(/^```\s*([\w+-]*)\s*$/);
    if (fence) {
      const codeLines: string[] = [];
      index += 1;
      while (index < lines.length && !/^```\s*$/.test(lines[index] ?? '')) {
        codeLines.push(lines[index] ?? '');
        index += 1;
      }
      if (index < lines.length) index += 1;
      blocks.push(
        <pre
          key={`block-${blockIndex++}`}
          className="overflow-x-auto rounded-md border bg-muted/50 p-3 text-sm"
        >
          <code className={fence[1] ? `language-${fence[1]}` : undefined}>
            {codeLines.join('\n')}
          </code>
        </pre>,
      );
      continue;
    }

    const heading = line.match(/^(#{1,6})\s+(.+)$/);
    if (heading) {
      const level = heading[1]?.length ?? 1;
      const key = `block-${blockIndex++}`;
      blocks.push(
        headingNode(
          level,
          inlineMarkdown(heading[2] ?? '', `heading-${blockIndex}`),
          key,
        ),
      );
      index += 1;
      continue;
    }

    if (/^---+$/.test(line)) {
      blocks.push(
        <hr key={`block-${blockIndex++}`} className="border-border" />,
      );
      index += 1;
      continue;
    }

    const quoteLines: string[] = [];
    while (index < lines.length && /^>\s?/.test(lines[index] ?? '')) {
      quoteLines.push((lines[index] ?? '').replace(/^>\s?/, ''));
      index += 1;
    }
    if (quoteLines.length > 0) {
      blocks.push(
        <blockquote
          key={`block-${blockIndex++}`}
          className="border-l-2 border-primary/50 pl-4 text-muted-foreground"
        >
          {inlineMarkdown(quoteLines.join('\n'), `quote-${blockIndex}`)}
        </blockquote>,
      );
      continue;
    }

    const unordered = /^[-*+]\s+/.test(line);
    const ordered = /^\d+\.\s+/.test(line);
    if (unordered || ordered) {
      const items: string[] = [];
      const pattern = unordered ? /^[-*+]\s+(.+)$/ : /^\d+\.\s+(.+)$/;
      while (index < lines.length) {
        const item = (lines[index] ?? '').match(pattern);
        if (!item) break;
        items.push(item[1] ?? '');
        index += 1;
      }
      const List = ordered ? 'ol' : 'ul';
      blocks.push(
        <List
          key={`block-${blockIndex++}`}
          className={ordered ? 'list-decimal pl-6' : 'list-disc pl-6'}
        >
          {items.map((item, itemIndex) => (
            <li key={`item-${itemIndex}`}>
              {inlineMarkdown(item, `list-${blockIndex}-${itemIndex}`)}
            </li>
          ))}
        </List>,
      );
      continue;
    }

    const paragraph: string[] = [line];
    index += 1;
    while (
      index < lines.length &&
      (lines[index] ?? '').trim() &&
      !isBlockStart(lines[index] ?? '')
    ) {
      paragraph.push(lines[index] ?? '');
      index += 1;
    }
    blocks.push(
      <p key={`block-${blockIndex++}`} className="leading-7">
        {inlineMarkdown(paragraph.join('\n'), `paragraph-${blockIndex}`)}
      </p>,
    );
  }

  return (
    <article className="mx-auto flex max-w-3xl flex-col gap-5 px-8 py-8 text-[15px] text-foreground">
      {blocks.length > 0 ? (
        blocks
      ) : (
        <p className="text-muted-foreground">This Markdown file is empty.</p>
      )}
    </article>
  );
}
