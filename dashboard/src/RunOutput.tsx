import { Fragment, type ReactNode } from "react";

const CODEX_THREAD_STARTED_PATTERN =
  /Started Codex thread ([0-9a-fA-F]{8}(?:-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12})\./g;

export function RunOutput({ output }: { output: string }) {
  const content: ReactNode[] = [];
  let precedingTextEnd = 0;

  for (const match of output.matchAll(CODEX_THREAD_STARTED_PATTERN)) {
    const matchStart = match.index;
    const threadId = match[1];
    content.push(output.slice(precedingTextEnd, matchStart));
    content.push(
      <a
        className="codex-thread-link"
        href={`codex://threads/${threadId}`}
        key={matchStart}
        title="Open thread in Codex"
      >
        {match[0]}
      </a>,
    );
    precedingTextEnd = matchStart + match[0].length;
  }
  content.push(output.slice(precedingTextEnd));

  return <Fragment>{content}</Fragment>;
}
