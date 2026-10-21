import { Fragment } from 'react';
import { inlineCodeParts } from './lib/inlineCode';

export function InlineCodeText({ text }: { text: string }) {
  return <>{inlineCodeParts(text).map((part, index) => part.kind === 'code'
    ? <code key={index}>{part.text}</code>
    : <Fragment key={index}>{part.text}</Fragment>)}</>;
}
