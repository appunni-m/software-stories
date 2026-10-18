import { useEffect, useId, useRef, useState, type FormEvent } from 'react';
import { Button, Dialog, Flex, Text, TextField } from '@radix-ui/themes';
import { repairChapterCitation, type ChapterSourceAnchor } from './lib/commands';
import { errorMessageOr } from './lib/errorMessage';

export function ChapterCitationEditor({projectId,baseRevisionId,snapshotId,anchor,paths,disabled,open,onOpenChange,onApply}:{
  projectId:string;baseRevisionId:string;snapshotId:string;anchor:ChapterSourceAnchor;paths:string[];disabled:boolean;
  open:boolean;onOpenChange:(open:boolean)=>void;onApply:(anchor:ChapterSourceAnchor)=>void;
}) {
  const [path,setPath]=useState(anchor.path),[from,setFrom]=useState(String(anchor.startLine)),[to,setTo]=useState(String(anchor.endLine));
  const [busy,setBusy]=useState(false),[error,setError]=useState<string|null>(null);
  const request=useRef(0),list=useId();
  useEffect(()=>{
    ++request.current;
    if(open){setPath(anchor.path);setFrom(String(anchor.startLine));setTo(String(anchor.endLine));setBusy(false);setError(null);}
    return()=>{++request.current;};
  },[open,projectId,baseRevisionId,snapshotId,anchor.anchorId]);
  const changeOpen=(next:boolean)=>{if(!next)++request.current;onOpenChange(next);};
  const apply=async(event:FormEvent<HTMLFormElement>)=>{
    event.preventDefault();if(busy||disabled)return;
    const start=Number(from),end=Number(to);
    if(!path.trim()||!/^\d+$/.test(from)||!/^\d+$/.test(to)||!Number.isSafeInteger(start)||!Number.isSafeInteger(end)||start<1||end<start||end>0xffffffff){setError('Choose a file and an ordered range of positive line numbers.');return;}
    const ticket=++request.current;setBusy(true);setError(null);
    try{
      const next=await repairChapterCitation(projectId,baseRevisionId,anchor.anchorId,path.trim(),start,end);
      if(ticket!==request.current)return;
      if(next.anchorId!==anchor.anchorId||next.snapshotId!==snapshotId)throw new Error('The source link did not match this chapter. Your draft is unchanged.');
      onApply(next);changeOpen(false);
    }catch(failure){if(ticket===request.current)setError(errorMessageOr(failure,'This source range could not be opened. Your draft is unchanged.'));}
    finally{if(ticket===request.current)setBusy(false);}
  };
  return <Dialog.Root open={open} onOpenChange={changeOpen}>
    <Dialog.Trigger><Button size="1" variant="ghost" disabled={disabled}>Change citation</Button></Dialog.Trigger>
    <Dialog.Content maxWidth="560px">
      <Dialog.Title>Choose supporting code</Dialog.Title>
      <Dialog.Description size="2">Choose the code that supports this claim. Review its narration and visuals after changing the citation.</Dialog.Description>
      <form onSubmit={event=>void apply(event)}>
        <Flex direction="column" gap="3" mt="4">
          <label><Text as="div" size="2" mb="1">File</Text><TextField.Root aria-label="Citation file" list={list} value={path} maxLength={4096} disabled={busy} required onChange={event=>setPath(event.target.value)}/><datalist id={list}>{paths.map(p=><option key={p} value={p}/>)}</datalist></label>
          <Flex gap="3" wrap="wrap">
            <label><Text as="div" size="2" mb="1">From line</Text><TextField.Root aria-label="Citation from line" type="number" min="1" max="4294967295" step="1" required value={from} disabled={busy} onChange={event=>setFrom(event.target.value)}/></label>
            <label><Text as="div" size="2" mb="1">To line</Text><TextField.Root aria-label="Citation to line" type="number" min="1" max="4294967295" step="1" required value={to} disabled={busy} onChange={event=>setTo(event.target.value)}/></label>
          </Flex>
          {error&&<Text as="p" size="2" color="red" role="alert">{error}</Text>}
          <Flex justify="end" gap="3" mt="2"><Dialog.Close><Button type="button" variant="soft" color="gray">Cancel</Button></Dialog.Close><Button type="submit" disabled={busy||disabled}>{busy?'Reading source…':'Use these lines'}</Button></Flex>
        </Flex>
      </form>
    </Dialog.Content>
  </Dialog.Root>;
}
