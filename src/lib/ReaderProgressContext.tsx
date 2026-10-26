import { createContext, useContext } from 'react';
import type { SectionIdentity, StoryProgress } from './readerProgress';

/** The dedicated player shares its originating exploration's reading metadata.
 * Standalone browser previews keep their own section-scoped reader position. */
interface ReaderProgressAccess {
  story:(identity:SectionIdentity)=>{
  progress:StoryProgress|undefined;
  onProgressChange:(progress:StoryProgress)=>void;
  }|undefined;
  section:(revisionId:string)=>{
    sectionId:string|undefined;
    onSectionChange:(sectionId:string)=>void;
  }|undefined;
}
export const ReaderProgressContext = createContext<ReaderProgressAccess|null>(null);

export function useReaderProgress(identity:SectionIdentity) {
  return useContext(ReaderProgressContext)?.story(identity);
}

export function useReaderSection(revisionId:string|undefined) {
  const access=useContext(ReaderProgressContext);
  return revisionId?access?.section(revisionId):undefined;
}

/** Read a saved section for a revision selected outside the current reader route. */
export function useReaderSectionAccessor() {
  return useContext(ReaderProgressContext)?.section;
}
