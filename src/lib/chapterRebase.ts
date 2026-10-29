import type {ChapterRevision,ChapterSection} from './commands';

export type ChapterRebaseChoice='draft'|'latest';
export type ChapterRebaseChoices=Record<string,ChapterRebaseChoice|undefined>;

export interface ChapterRebaseChoiceState {
  base:ChapterRevision;
  latest:ChapterRevision;
  draft:ChapterRevision;
  choices:ChapterRebaseChoices;
}

/** Conflict decisions belong to the precise comparison that produced them. */
export function chapterRebaseChoicesFor(
  state:ChapterRebaseChoiceState|null,
  base:ChapterRevision,
  latest:ChapterRevision,
  draft:ChapterRevision,
):ChapterRebaseChoices {
  return state?.base===base&&state.latest===latest&&state.draft===draft?state.choices:{};
}

export interface ChapterRebaseConflict {
  id:string;
  label:string;
  draftSummary:string;
  latestSummary:string;
  draftDetail:string;
  latestDetail:string;
}

export interface ChapterRebaseResult {
  revision:ChapterRevision;
  conflicts:ChapterRebaseConflict[];
  unresolved:string[];
}

const same=(left:unknown,right:unknown)=>JSON.stringify(left)===JSON.stringify(right);
const copy=<T,>(value:T):T=>value===undefined?value:structuredClone(value);

function summary(value:unknown):string {
  if(value===undefined)return 'Remove this section';
  if(value===null)return 'None';
  if(typeof value==='string')return value.trim()?value.trim().replace(/\s+/g,' ').slice(0,140):'Empty';
  if(typeof value==='number'||typeof value==='boolean')return String(value);
  if(Array.isArray(value))return `${value.length} item${value.length===1?'':'s'}`;
  return 'Updated';
}

function detail(value:unknown):string {
  if(value===undefined)return 'Removed in this version.';
  if(value===null)return 'None';
  if(typeof value==='string')return value;
  try{return JSON.stringify(value,null,2)??String(value);}
  catch{return String(value);}
}

function mergeValue<T>(
  id:string,label:string,base:T,draft:T,latest:T,
  choices:ChapterRebaseChoices,conflicts:ChapterRebaseConflict[],
):T {
  if(same(draft,base))return copy(latest);
  if(same(latest,base)||same(draft,latest))return copy(draft);
  conflicts.push({id,label,draftSummary:summary(draft),latestSummary:summary(latest),draftDetail:detail(draft),latestDetail:detail(latest)});
  return copy(choices[id]==='draft'?draft:latest);
}

const sectionFields=['order','title','prose','spokenText','claims','dependencies','visual','hotspots'] as const;
type MergeableSectionField=typeof sectionFields[number];
const sameSectionContent=(left:ChapterSection,right:ChapterSection)=>
  left.sectionId===right.sectionId&&sectionFields.every(field=>same(left[field],right[field]));

function mergeSection(
  base:ChapterSection,draft:ChapterSection,latest:ChapterSection,
  choices:ChapterRebaseChoices,conflicts:ChapterRebaseConflict[],
):ChapterSection {
  const merged={...latest};
  for(const field of sectionFields){
    const value=mergeValue<ChapterSection[MergeableSectionField]>(
      `${base.sectionId}:${field}`,
      `${base.title} · ${field==='spokenText'?'narration':field}`,
      base[field],draft[field],latest[field],choices,conflicts,
    );
    // Section fields are independent edit units; identity and revision IDs stay with the newer base.
    (merged as unknown as Record<string,unknown>)[field]=value;
  }
  return merged;
}

/** Three-way rebase of editable chapter text. Arrays with linked evidence are
 * atomic fields, so overlapping changes require an explicit draft/latest choice. */
export function rebaseChapterDraft(
  base:ChapterRevision,latest:ChapterRevision,draft:ChapterRevision,
  choices:ChapterRebaseChoices={},
):ChapterRebaseResult {
  const sameTarget=base.projectId===latest.projectId&&latest.projectId===draft.projectId
    &&base.snapshotId===latest.snapshotId&&latest.snapshotId===draft.snapshotId
    &&base.chapterId===latest.chapterId&&latest.chapterId===draft.chapterId
    &&base.target.targetId===latest.target.targetId&&latest.target.targetId===draft.target.targetId
    &&same(base.target.scope,latest.target.scope)&&same(latest.target.scope,draft.target.scope);
  if(!sameTarget)throw new Error('These chapter versions do not share the same project, snapshot and target.');

  const conflicts:ChapterRebaseConflict[]=[];
  const title=mergeValue('chapter:title','Chapter · title',base.title,draft.title,latest.title,choices,conflicts);
  const baseSections=new Map(base.sections.map(section=>[section.sectionId,section]));
  const draftSections=new Map(draft.sections.map(section=>[section.sectionId,section]));
  const latestSections=new Map(latest.sections.map(section=>[section.sectionId,section]));
  const ids=new Set([...baseSections.keys(),...draftSections.keys(),...latestSections.keys()]);
  const sections:ChapterSection[]=[];

  for(const id of ids){
    const original=baseSections.get(id),mine=draftSections.get(id),newer=latestSections.get(id);
    if(!original){
      if(mine&&newer){
        if(sameSectionContent(mine,newer)){sections.push(copy(newer));continue;}
        const conflictId=`${id}:new`;
        conflicts.push({id:conflictId,label:`New section · ${mine.title}`,draftSummary:summary(mine.title),latestSummary:summary(newer.title),draftDetail:detail(mine),latestDetail:detail(newer)});
        sections.push(copy(choices[conflictId]==='draft'?mine:newer));
      }else if(mine||newer)sections.push(copy((mine??newer)!));
      continue;
    }
    if(!mine&&!newer)continue;
    if(!mine){
      if(sameSectionContent(original,newer!))continue;
      const conflictId=`${id}:presence`;
      conflicts.push({id:conflictId,label:`${original.title} · section`,draftSummary:'Remove this section',latestSummary:summary(newer!.title),draftDetail:'This draft removes the section.',latestDetail:detail(newer)});
      if(choices[conflictId]!=='draft')sections.push(copy(newer!));
      continue;
    }
    if(!newer){
      if(sameSectionContent(original,mine))continue;
      const conflictId=`${id}:presence`;
      conflicts.push({id:conflictId,label:`${original.title} · section`,draftSummary:summary(mine.title),latestSummary:'Remove this section',draftDetail:detail(mine),latestDetail:'This revision removes the section.'});
      if(choices[conflictId]==='draft')sections.push(copy(mine));
      continue;
    }
    sections.push(mergeSection(original,mine,newer,choices,conflicts));
  }

  sections.sort((left,right)=>left.order-right.order||left.sectionId.localeCompare(right.sectionId));
  // Independent insertions can both choose the same position. Keep their
  // deterministic merged order, then restore the chapter's required
  // contiguous zero-based positions before the candidate reaches validation.
  const orderedSections=sections.map((section,order)=>({...section,order}));
  const revision:ChapterRevision={...copy(latest),title,sections:orderedSections};
  return {revision,conflicts,unresolved:conflicts.filter(conflict=>!choices[conflict.id]).map(conflict=>conflict.id)};
}
