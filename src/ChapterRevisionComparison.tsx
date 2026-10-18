import type {ChapterRevision} from './lib/commands';
import {compareChapterRevisions,revisionChangeContext,type RevisionChange,type RevisionChangeEntity} from './lib/revisionHistory';

interface DiffValue {
  field:string;
  context?:string;
  before:string;
  after:string;
  beforeDetail?:string;
  afterDetail?:string;
}
const entityLabels:Record<RevisionChangeEntity,string>={
  chapter:'chapter',section:'section',claim:'claim',anchor:'source reference',dependency:'dependency',
  visual:'visual',visualNode:'visual node',visualEdge:'visual connection',presentation:'visual presentation',
  scene:'scene',sceneElement:'visual element',hotspot:'link',
};
const fieldLabels:Record<string,string>={
  startByte:'start byte',endByteInclusive:'end byte',startLine:'start line',endLine:'end line',
  startColumn:'start column',endColumn:'end column',sourceSha256:'source hash',evidenceAnchorIds:'source references',
  spokenText:'spoken narration',targetScope:'target',claimOrder:'claim order',nodeOrder:'node order',
  claimIndex:'scene position',nodeIndex:'visual node position',nodeId:'linked visual node',parentIndex:'parent element',
  valueOrigin:'value origin',inferenceLabel:'inference label',inferenceBasis:'inference basis',
};
function changeRow(change:RevisionChange,before:ChapterRevision,after:ChapterRevision):DiffValue {
  const action=change.kind[0].toUpperCase()+change.kind.slice(1);
  const field=change.field?` · ${fieldLabels[change.field]??change.field.replace(/[A-Z]/g,letter=>` ${letter.toLowerCase()}`)}`:'';
  return {field:`${action} ${entityLabels[change.entity]}${field}`,context:revisionChangeContext(change,before,after),
    before:change.beforeSummary??'—',after:change.afterSummary??'—',beforeDetail:change.beforeDetail,afterDetail:change.afterDetail};
}

/** Read-only comparison; authoring and acceptance stay in the owning editor. */
export function ChapterRevisionComparison({before,after}:{before:ChapterRevision;after:ChapterRevision}) {
  const diff=compareChapterRevisions(before,after);
  const metadata:DiffValue[]=[
    ...(before.snapshotId!==after.snapshotId?[{field:'Source snapshot',before:before.snapshotId,after:after.snapshotId}]:[]),
    ...(before.lifecycle!==after.lifecycle?[{field:'Status',before:before.lifecycle,after:after.lifecycle}]:[]),
    ...(before.basedOnRevisionId!==after.basedOnRevisionId?[{field:'Parent revision',before:before.basedOnRevisionId??'—',after:after.basedOnRevisionId??'—'}]:[]),
  ];
  if(!metadata.length&&diff.identical)return <p className="chapter-revision-no-diff">No chapter text, source references or visual definitions changed between these revisions.</p>;
  return <div className="chapter-revision-diff">
    {!!metadata.length&&<div className="chapter-revision-diff-group"><h5>Chapter</h5>{metadata.map(value=><DiffRow key={value.field} value={value}/>)}</div>}
    {!!diff.changes.length&&<div className="chapter-revision-diff-group"><h5>Content · {diff.counts.total} change{diff.counts.total===1?'':'s'}</h5>{diff.changes.map((change,index)=><DiffRow key={`${change.path}:${change.field}:${index}`} value={changeRow(change,before,after)}/>)}</div>}
  </div>;
}
function DiffRow({value}:{value:DiffValue}) {
  return <div className="chapter-revision-diff-row">
    <strong>{value.field}{value.context&&<small>{value.context}</small>}</strong>
    <div><span>Before</span><DiffText summary={value.before} detail={value.beforeDetail}/></div>
    <div><span>After</span><DiffText summary={value.after} detail={value.afterDetail}/></div>
  </div>;
}
function DiffText({summary,detail}:{summary:string;detail?:string}) {
  return detail?<details className="chapter-revision-full-text"><summary>{summary}</summary><pre>{detail}</pre></details>:<pre>{summary}</pre>;
}
