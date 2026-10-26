import {useMemo,useState} from 'react';
import type {ChapterRevision} from './lib/commands';
import {rebaseChapterDraft,type ChapterRebaseChoices} from './lib/chapterRebase';

export function ChapterRebaseReview({base,latest,draft,onCancel,onApply}:{
  base:ChapterRevision;latest:ChapterRevision;draft:ChapterRevision;
  onCancel:()=>void;onApply:(revision:ChapterRevision)=>void;
}){
  const [choices,setChoices]=useState<ChapterRebaseChoices>({});
  const result=useMemo(()=>rebaseChapterDraft(base,latest,draft,choices),[base,latest,draft,choices]);
  const applyChoiceToAll=(choice:'draft'|'latest')=>{
    const next:ChapterRebaseChoices={};
    for(const conflict of result.conflicts)next[conflict.id]=choice;
    setChoices(next);
  };
  return <section className="chapter-rebase-panel" aria-labelledby="chapter-rebase-title">
    <div className="chapter-rebase-heading">
      <div><strong id="chapter-rebase-title">Bring your draft up to date</strong>
        <p>Saved revision r{latest.revisionNumber} is newer than the version you edited. Changes made in only one version carry over. If both changed the same part, choose what to keep. Claims, citations, and visuals stay together. Review the combined draft before saving.</p>
      </div>
      <button className="text-button" type="button" onClick={onCancel}>Back to draft</button>
    </div>
    {result.conflicts.length===0
      ? <p className="chapter-rebase-clear" role="status">Your draft has no conflicting edits with saved r{latest.revisionNumber} and can be brought forward.</p>
      : <>
        {result.conflicts.length>1&&<div className="chapter-rebase-bulk-actions" role="group" aria-label="Apply one choice to every conflict">
          <span>For all {result.conflicts.length} conflicts, choose:</span>
          <button className="text-button" type="button" onClick={()=>applyChoiceToAll('draft')}>
            Keep my edits
          </button>
          <button className="text-button" type="button" onClick={()=>applyChoiceToAll('latest')}>
            Use saved version
          </button>
        </div>}
        <div className="chapter-rebase-conflicts" role="group" aria-label="Resolve overlapping edits">
        {result.conflicts.map(conflict=><fieldset key={conflict.id} className="chapter-rebase-conflict">
          <legend>{conflict.label}</legend>
          <div className="chapter-rebase-options">
            {(['draft','latest'] as const).map(choice=>{
              const isDraft=choice==='draft';
              const title=isDraft?'Your edits':`Saved r${latest.revisionNumber}`;
              const description=isDraft?conflict.draftSummary:conflict.latestSummary;
              const fullContent=isDraft?conflict.draftDetail:conflict.latestDetail;
              return <div key={choice} className="chapter-rebase-option">
                <label className={choices[conflict.id]===choice?'is-selected':''}>
                  <input type="radio" name={`rebase-${conflict.id}`} value={choice}
                    checked={choices[conflict.id]===choice}
                    onChange={()=>setChoices(current=>({...current,[conflict.id]:choice}))}/>
                  <span><strong>{title}</strong><small title={description}>{description}</small></span>
                </label>
                <details className="chapter-rebase-option-details"><summary>View full {isDraft?'draft':'saved'} content</summary><pre>{fullContent}</pre></details>
              </div>;
            })}
          </div>
        </fieldset>)}
        </div>
      </>}
    <div className="chapter-rebase-actions">
      {result.unresolved.length>0&&<span role="status">Choose what to keep for {result.unresolved.length} conflict{result.unresolved.length===1?'':'s'}.</span>}
      <button className="primary-button" type="button" disabled={result.unresolved.length>0}
        onClick={()=>onApply(result.revision)}>Continue with combined draft</button>
    </div>
  </section>;
}
