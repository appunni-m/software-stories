// Version the writing contract independently of the saved source and voice.
// A takeaway alone cannot distinguish an old animation commentary from a
// software explanation, so old scripts need rewriting rather than reuse.
export const narrativeVersion = 4;
export const maxNarrationWords = 160;
export const isCurrentNarrative = chapter => chapter?.narrativeVersion === narrativeVersion;
export const narratesTheScreen = text => /\b(?:watch|look at|follow)\s+(?:(?:the|this|your|that)\s+)?(?:blue|red|green|arrow|path|fork|animation|handoff|dot|marker)\b/i.test(text);
