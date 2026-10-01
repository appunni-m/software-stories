const words = value => String(value).toLowerCase().match(/[\p{L}\p{N}]+/gu) || [];

export function cueIndex(narration, cue) {
  const all = words(narration), part = words(cue);
  if (!part.length) return -1;
  return all.findIndex((_, i) => part.every((word, j) => all[i + j] === word));
}

export function timelineAt(chapter, progress) {
  const steps = chapter.scene.steps, duration = chapter.audio?.duration || 1;
  const length = words(chapter.narration).length;
  let alignedToSpeech = Boolean(chapter.audio?.alignment);
  const starts = steps.map((step, i) => {
    if (i === 0) return 0;
    const index = step.cue ? cueIndex(chapter.narration, step.cue) : -1;
    const aligned = index >= 0 ? chapter.audio?.alignment?.words?.[index]?.start : undefined;
    if (!Number.isFinite(aligned)) alignedToSpeech = false;
    return Number.isFinite(aligned) ? aligned / duration : index >= 0 ? index / length : i / steps.length;
  });
  // Old saved scenes and uncertain alignments must still have a stable timeline.
  for (let i = 1; i < starts.length; i++) starts[i] = Math.min(.99, Math.max(starts[i - 1] + .001, starts[i]));
  const p = Math.max(0, Math.min(1, progress));
  let index = starts.findLastIndex(start => p >= start);
  index = Math.max(0, index);
  const end = starts[index + 1] ?? 1;
  return { index, phase: Math.min(1, (p - starts[index]) / (end - starts[index])), starts, aligned: alignedToSpeech };
}

export const visualForms = {
  flow: { label: 'Responsibility map', purpose: 'Follow relationships between responsibilities.' },
  sequence: { label: 'Request timeline', purpose: 'Follow the order in which work moves between participants.' },
  branch: { label: 'Decision paths', purpose: 'See a condition and the paths it allows or rejects.' },
  state: { label: 'State transitions', purpose: 'See what changes after each event.' },
  memory: { label: 'Storage and reuse', purpose: 'See what is stored, retrieved, and reused.' },
  diff: { label: 'Before and after', purpose: 'Compare the earlier and current implementation.' },
  code: { label: 'Source close-up', purpose: 'Connect a focused explanation to the actual source.' },
};
