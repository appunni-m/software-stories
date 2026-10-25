import {useMemo,useState} from 'react';
import type {ChapterRevision} from './lib/commands';
import {rebaseChapterDraft,type ChapterRebaseChoices} from './lib/chapterRebase';

export function ChapterRebaseReview({base,latest,draft,onCancel,onApply}:{
  base:ChapterRevision;latest:ChapterRevision;draft:ChapterRevision;
  onCancel:()=>void;onApply:(revision:ChapterRevision)=>void;
}){
  const [choices,setChoices]=useState<ChapterRebaseChoices>({});
  const result=useMemo(()=>rebaseChapterDraft(base,latest,draft,choices),[base,latest,draft,choices]);
  return <section className="chapter-rebase-panel" aria-labelledby="chapter-rebase-title">
    <div className="chapter-rebase-heading">
      <div><strong id="chapter-rebase-title">Rebase your draft onto r{latest.revisionNumber}</strong>
        <p>Text changes made in only one version carry over. Linked claims, citations, and visuals stay together; if both versions changed one, choose which version to keep. Nothing is saved until you review and save the new draft.</p>
      </div>
      <button className="text-button" type="button" onClick={onCancel}>Back to draft</button>
    </div>
    {result.conflicts.length===0
      ? <p className="chapter-rebase-clear" role="status">No overlapping edits. Your changes can be carried onto r{latest.revisionNumber}.</p>
      : <div className="chapter-rebase-conflicts" role="group" aria-label="Resolve overlapping edits">
        {result.conflicts.map(conflict=><fieldset key={conflict.id} className="chapter-rebase-conflict">
          <legend>{conflict.label}</legend>
          <div className="chapter-rebase-options">
            {(['draft','latest'] as const).map(choice=>{
              const isDraft=choice==='draft';
              const title=isDraft?'Your draft':`Saved r${latest.revisionNumber}`;
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
      </div>}
    <div className="chapter-rebase-actions">
      {result.unresolved.length>0&&<span role="status">Choose a version for {result.unresolved.length} overlapping field{result.unresolved.length===1?'':'s'}.</span>}
      <button className="primary-button" type="button" disabled={result.unresolved.length>0}
        onClick={()=>onApply(result.revision)}>Use these changes</button>
    </div>
  </section>;
}
