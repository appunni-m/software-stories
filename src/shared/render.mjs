import { timelineAt, visualForms } from './timeline.mjs';
export const escape = value => String(value).replace(/[&<>"']/g, char => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[char]));
const C = { ink:'#172b45', muted:'#4d6075', blue:'#245bd6', line:'#bac9dc', pale:'#eaf1ff', white:'#ffffff', green:'#13715a', red:'#b63749' };
const clamp = value => Math.max(0, Math.min(1, value));
function lines(value, max) {
  const result = []; let line = '';
  for (const word of String(value).split(/\s+/)) {
    if ((line + ' ' + word).length > max && line) { result.push(line); line = ''; }
    line += (line ? ' ' : '') + word;
  }
  if (line) result.push(line);
  return result;
}
function text(value, x, y, { size=24, color=C.ink, max=40, limit=3, weight=400, anchor='start' }={}) {
  const wrapped = lines(value, max), shown = wrapped.slice(0,limit);
  if (wrapped.length > limit) shown[limit-1] = shown[limit-1].replace(/…$/, '') + '…';
  return `<text x="${x}" y="${y}" fill="${color}" font-size="${size}" font-weight="${weight}" text-anchor="${anchor}">${shown.map((line,i)=>`<tspan x="${x}" dy="${i?size*1.3:0}">${escape(line)}</tspan>`).join('')}</text>`;
}
const rect = (x,y,w,h,fill=C.white,stroke=C.line,r=16) => `<rect x="${x}" y="${y}" width="${w}" height="${h}" rx="${r}" fill="${fill}" stroke="${stroke}"/>`;
function interactive(node, content, m) {
  return `<g data-node="${escape(node.id)}" role="button" tabindex="${m.locked?-1:0}" aria-disabled="${m.locked}" aria-label="Explore ${escape(node.label)}"><title>${escape(node.label)} — ${escape(node.detail)}</title>${content}</g>`;
}
function trace(x1,y1,x2,y2,phase,on,label='',m) {
  const cx=(x1+x2)/2, cy=(y1+y2)/2, t=clamp(phase);
  return `<path d="M${x1},${y1} L${x2},${y2}" stroke="${on?C.blue:C.line}" stroke-width="${on?3:2}" fill="none" marker-end="url(#arrow)"/>${on?`<circle data-motion="handoff" cx="${x1+(x2-x1)*t}" cy="${y1+(y2-y1)*t}" r="8" fill="${C.blue}" stroke="white" stroke-width="3"/>`:''}${label?rect(cx-65,cy-27,130,28,'#f7faff','none',5)+text(label,cx,cy-7,{size:m.small,anchor:'middle',max:18,limit:1}):''}`;
}
function flow(scene, timing, m) {
  const focus=scene.nodes.findIndex(n=>n.id===scene.steps[timing.index].nodeId), start=scene.nodes.length>6?Math.min(scene.nodes.length-6,Math.max(0,focus-2)):0;
  const visible=scene.nodes.slice(start,start+6);
  const columns=m.compact?2:Math.min(3,visible.length), rows=Math.ceil(visible.length/columns);
  const cell=m.bodyWidth/columns, row=m.bodyHeight/rows;
  const positions=new Map(visible.map((node,i)=>[node.id,{x:m.left+cell*(i%columns+.5), y:m.top+row*(Math.floor(i/columns)+.5)-22}]));
  const active=scene.steps[timing.index].nodeId, from=scene.steps[timing.index].fromNodeId;
  let out=scene.edges.map(edge=>{
    const a=positions.get(edge.from), b=positions.get(edge.to); if(!a||!b)return ''; const dx=b.x-a.x,dy=b.y-a.y,len=Math.hypot(dx,dy)||1;
    return trace(a.x+dx/len*60,a.y+dy/len*60,b.x-dx/len*66,b.y-dy/len*66,clamp(timing.phase/.65),edge.to===active&&(from===undefined||from===edge.from),edge.label,m);
  }).join('');
  out+=visible.map((node,i)=>{
    const p=positions.get(node.id),on=node.id===active;
    return interactive(node,`<circle cx="${p.x}" cy="${p.y}" r="${on?52+Math.sin(timing.phase*Math.PI)*5:48}" fill="${on?C.pale:C.white}" stroke="${on?C.blue:C.line}" stroke-width="${on?3:2}"/>${text(String(i+1).padStart(2,'0'),p.x,p.y+12,{size:32,color:on?C.blue:C.muted,anchor:'middle',weight:600})}${rect(p.x-cell*.44,p.y+60,cell*.88,80,'#f7faff','none',8)}${text(node.label,p.x,p.y+85,{size:m.font,weight:600,anchor:'middle',max:Math.floor(cell/(m.font*.58))-2,limit:2})}${m.compact&&!m.recap?text(node.detail,p.x,p.y+85+m.font*2.7,{size:m.small,color:C.muted,anchor:'middle',max:Math.floor(cell/(m.small*.56))-3,limit:2}):''}`,m);
  }).join('');
  return `<g data-visual="relationship-map">${out}</g>`;
}
function sequence(scene,timing,m) {
  const active=scene.steps[timing.index].nodeId;
  // A focus window keeps lane names legible for larger and mobile scenes.
  const current=scene.nodes.findIndex(n=>n.id===active), count=Math.min(scene.nodes.length,m.compact?2:4);
  const start=Math.max(0,Math.min(scene.nodes.length-count,current-Math.floor(count/2)));
  const shown=scene.nodes.slice(start,start+count), cell=m.bodyWidth/count;
  const positions=new Map(shown.map((node,i)=>[node.id,m.left+cell*(i+.5)]));
  const laneTop=m.top+90,bottom=m.top+m.bodyHeight-16;
  let out=shown.map(node=>{
    const x=positions.get(node.id),on=node.id===active;
    return interactive(node,rect(x-cell*.44,m.top,cell*.88,76,on?C.pale:C.white,on?C.blue:C.line,12)+text(node.label,x,m.top+31,{size:m.font,weight:600,max:Math.floor(cell/(m.font*.58))-2,limit:2,anchor:'middle'}),m)+`<path d="M${x} ${laneTop} V${bottom}" stroke="${C.line}" stroke-width="2" stroke-dasharray="5 9"/>`;
  }).join('');
  const visibleSteps=scene.steps.slice(Math.max(0,timing.index-2),Math.max(0,timing.index-2)+(m.compact?3:5));
  out+=visibleSteps.map((step,i)=>{
    const absolute=scene.steps.indexOf(step),prev=scene.steps[Math.max(0,absolute-1)];
    const x2=positions.get(step.nodeId), x1=positions.get(step.fromNodeId===undefined?prev.nodeId:step.fromNodeId);
    if(x2===undefined) return '';
    const y=laneTop+40+i*(m.compact?135:Math.min(67,(m.bodyHeight-140)/visibleSteps.length));
    const on=absolute===timing.index;
    const begin=x1??m.left, finish=x2===begin?x2+Math.min(50,cell*.2):x2;
    return (step.fromNodeId===null?`<circle cx="${x2}" cy="${y}" r="9" fill="${on?C.blue:C.line}"/>`:trace(begin,y,finish,y,clamp(timing.phase/.65),on,'',m))+text(step.label,m.left+8,y-14,{size:m.small,color:on?C.blue:C.muted,max:Math.floor(m.bodyWidth/(m.small*.57)),limit:1});
  }).join('');
  if(scene.nodes.length>count)out+=text(`Focus ${start+1}–${start+count} of ${scene.nodes.length} participants`,m.left,bottom+24,{size:m.small,color:C.muted});
  return `<g data-visual="lifelines">${out}</g>`;
}
function branch(scene,timing,m) {
  const active=scene.steps[timing.index].nodeId, from=scene.steps[timing.index].fromNodeId;
  const incoming=new Set(scene.edges.map(edge=>edge.to));
  const depth=new Map(scene.nodes.map(node=>[node.id,incoming.has(node.id)?1:0]));
  // Relax a bounded DAG. Cyclic scenes remain displayable without looping.
  for(let pass=0;pass<scene.nodes.length;pass++)for(const edge of scene.edges){
    if(depth.get(edge.to)<=depth.get(edge.from))depth.set(edge.to,Math.min(scene.nodes.length-1,depth.get(edge.from)+1));
  }
  const levels=[...new Set(depth.values())].sort((a,b)=>a-b);
  const points=new Map();
  for(const [levelIndex,level]of levels.entries()){
    const items=scene.nodes.filter(n=>depth.get(n.id)===level);
    for(const [i,node]of items.entries())points.set(node.id,m.compact?{x:m.left+m.bodyWidth*(i+.5)/items.length,y:m.top+75+levelIndex*(m.bodyHeight-130)/Math.max(1,levels.length-1)}:{x:m.left+110+levelIndex*(m.bodyWidth-220)/Math.max(1,levels.length-1),y:m.top+m.bodyHeight*(i+.5)/items.length});
  }
  let out=scene.edges.map(edge=>{const a=points.get(edge.from),b=points.get(edge.to);return trace(a.x,a.y,b.x,b.y,clamp(timing.phase/.65),edge.to===active&&(from===undefined||from===edge.from),edge.label,m);}).join('');
  out+=scene.nodes.map(node=>{
    const p=points.get(node.id),on=node.id===active,split=scene.edges.filter(e=>e.from===node.id).length>1;
    const width=m.compact?255:200,h=m.compact?96:84;
    const shape=split?`<path d="M${p.x-width/2} ${p.y} L${p.x} ${p.y-h*.8} L${p.x+width/2} ${p.y} L${p.x} ${p.y+h*.8} Z" fill="${on?C.pale:C.white}" stroke="${on?C.blue:C.line}" stroke-width="3"/>`:rect(p.x-width/2,p.y-h/2,width,h,on?C.pale:C.white,on?C.blue:C.line,h/2);
    return interactive(node,shape+text(node.label,p.x,p.y-5,{size:m.font,weight:600,max:Math.floor(width/(m.font*.58))-2,limit:2,anchor:'middle'}),m);
  }).join('');
  return `<g data-visual="condition-paths">${out}</g>`;
}
function state(scene,timing,m) {
  const step=scene.steps[timing.index], current=scene.nodes.find(n=>n.id===step.nodeId),previous=scene.steps[timing.index-1];
  const x=m.left+m.bodyWidth/2,y=m.top+(m.compact?240:170),r=m.compact?155:122;
  const sweep=2*Math.PI*r*clamp(timing.phase);
  let out=`<circle cx="${x}" cy="${y}" r="${r}" fill="${C.white}" stroke="${C.line}" stroke-width="3"/><circle data-motion="transition" cx="${x}" cy="${y}" r="${r}" fill="none" stroke="${C.blue}" stroke-width="7" stroke-dasharray="${sweep} ${2*Math.PI*r}" transform="rotate(-90 ${x} ${y})"/>`;
  out+=interactive(current,text(current.label,x,y-17,{size:m.font+3,weight:600,anchor:'middle',max:19,limit:2})+text(current.detail,x,y+58,{size:m.small,color:C.muted,max:22,limit:3,anchor:'middle'}),m);
  if(previous)out+=text(`After: ${previous.label}`,x,m.top+22,{size:m.small,color:C.muted,max:50,limit:1,anchor:'middle'});
  const cell=m.bodyWidth/scene.nodes.length,trackY=m.top+m.bodyHeight-18;
  out+=scene.nodes.map((node,i)=>{const nx=m.left+cell*(i+.5),on=node.id===current.id;return interactive(node,`<circle cx="${nx}" cy="${trackY}" r="${on?10:6}" fill="${on?C.blue:C.line}"/>`+text(String(i+1),nx,trackY+32,{size:m.small,anchor:'middle'}),m);}).join('');
  out+=`<path d="M${m.left+cell/2} ${trackY} H${m.left+m.bodyWidth-cell/2}" stroke="${C.line}" stroke-width="2"/>`;
  return `<g data-visual="state-transition">${out}</g>`;
}
function memory(scene,timing,m) {
  const active=scene.steps[timing.index].nodeId,count=scene.nodes.length;
  const left=m.left+(m.compact?0:m.bodyWidth*.35),width=m.compact?m.bodyWidth:m.bodyWidth*.65;
  const gap=12,h=(m.bodyHeight-gap*(count-1))/count;
  let out='';
  if(!m.compact){
    out+=text('A reusable explanation',m.left,m.top+60,{size:30,weight:600,max:18,limit:2});
    out+=text('Read the stored work. Generate only what is missing.',m.left,m.top+160,{size:24,color:C.muted,max:20,limit:4});
  }
  out+=scene.nodes.map((node,i)=>{
    const y=m.top+i*(h+gap),on=node.id===active;
    const motion=on?Math.sin(timing.phase*Math.PI)*18:0;
    const source=left+motion;
    return interactive(node,rect(source,y,width-motion,h,on?C.pale:C.white,on?C.blue:C.line,10)+`<path d="M${source+18} ${y+12} V${y+h-12}" stroke="${on?C.blue:C.line}" stroke-width="5"/>`+text(node.label,source+38,y+(m.compact?45:35),{size:m.font,weight:600,max:Math.floor((width-65)/(m.font*.58)),limit:1})+text(node.detail,source+38,y+(m.compact?87:65),{size:m.small,color:C.muted,max:Math.floor((width-65)/(m.small*.56)),limit:m.compact?2:1}),m);
  }).join('');
  return `<g data-visual="storage-slots">${out}</g>`;
}
function evidenceFor(chapter,index) { return chapter.claims[chapter.scene.steps[index].claimIndex??0]?.evidence[0]??chapter.claims[0].evidence[0]; }
function sourcePane(evidence,x,y,width,height,m,focus=0,other) {
  if(!evidence)return rect(x,y,width,height,'#f3f6fa')+text('Earlier source not provided',x+24,y+60,{size:m.font,max:25,limit:2})+text('This view cannot establish the previous behavior.',x+24,y+155,{size:m.small,color:C.muted,max:25});
  const list=evidence.quote.split('\n'),lineHeight=m.compact?40:29,capacity=Math.floor((height-90)/lineHeight);
  const offset=Math.max(0,Math.min(list.length-capacity,focus-Math.floor(capacity/2)));
  const shown=list.slice(offset,offset+capacity),others=new Set(other?.quote.split('\n'));
  return rect(x,y,width,height)+text(evidence.path,x+20,y+32,{size:m.small,color:C.blue,max:Math.floor((width-45)/(m.small*.57)),limit:1})+shown.map((line,i)=>{
    const n=offset+i,on=n===focus,changed=other&&!others.has(line),base=y+78+i*lineHeight;
    const fit=Math.floor((width-82)/(m.small*.6));
    return (on||changed?rect(x+8,base-lineHeight+8,width-16,lineHeight,on?C.pale:'#fff0f0','none',4):'')+`<text x="${x+17}" y="${base}" fill="${C.muted}" font-family="monospace" font-size="${m.small-2}">${evidence.startLine+n}</text><text x="${x+62}" y="${base}" fill="${C.ink}" font-family="monospace" font-size="${m.small}" xml:space="preserve">${escape(line.length>fit?line.slice(0,fit-1)+'…':line)}</text>`;
  }).join('')+(list.length>capacity?text(`Lines ${evidence.startLine+offset}–${evidence.startLine+offset+shown.length-1} · open source for the full quote`,x+20,y+height-12,{size:m.small-3,color:C.muted,max:70,limit:1}):'');
}
function code(chapter,timing,m) {
  const evidence=evidenceFor(chapter,timing.index),step=chapter.scene.steps[timing.index],node=chapter.scene.nodes.find(n=>n.id===step.nodeId);
  const focus=Math.min(evidence.quote.split('\n').length-1,Math.floor(timing.phase*evidence.quote.split('\n').length));
  const width=m.compact?m.bodyWidth:m.bodyWidth*.68,height=m.compact?m.bodyHeight*.64:m.bodyHeight;
  const pane=sourcePane(evidence,m.left,m.top,width,height,m,focus);
  const x=m.compact?m.left:m.left+width+30,y=m.compact?m.top+height+45:m.top+50;
  return `<g data-visual="source-lens">${pane}${interactive(node,text(node.label,x,y,{size:m.font,weight:600,max:m.compact?35:17,limit:2})+text(step.detail,x,y+m.font*2.9,{size:m.small,color:C.muted,max:m.compact?36:24,limit:4}),m)}</g>`;
}
function diff(chapter,timing,m) {
  const evidence=chapter.claims.flatMap(c=>c.evidence),before=evidence.find(e=>e.path.startsWith('before:')),after=evidence.find(e=>!e.path.startsWith('before:'));
  const width=m.compact?m.bodyWidth:(m.bodyWidth-30)/2,height=m.compact?(m.bodyHeight-70)/2:m.bodyHeight-45;
  const secondX=m.compact?m.left:m.left+width+30,secondY=m.compact?m.top+height+70:m.top+45;
  return `<g data-visual="source-comparison">${text('BEFORE',m.left,m.top+10,{size:m.small,color:C.red,weight:600})}${sourcePane(before,m.left,m.top+35,width,height,m,Math.floor(timing.phase*4),after)}${text('AFTER',secondX,m.compact?secondY-14:m.top+10,{size:m.small,color:C.green,weight:600})}${sourcePane(after,secondX,secondY,width,height,m,Math.floor(timing.phase*4),before)}</g>`;
}
export function renderScene(chapter, progress = 0, { compact = false, locked = false } = {}) {
  const scene = chapter.scene, timing = timelineAt(chapter,progress), step = scene.steps[timing.index];
  const m={compact,locked,recap:Boolean(chapter.takeaway),width:compact?720:1280,height:compact?1160:720,left:compact?34:55,top:compact?150:145,bodyWidth:compact?652:1170,bodyHeight:compact?720:365,font:compact?34:25,small:compact?27:21};
  const body = scene.type === 'sequence' ? sequence(scene,timing,m) : scene.type === 'branch' ? branch(scene,timing,m) : scene.type === 'state' ? state(scene,timing,m) : scene.type === 'memory' ? memory(scene,timing,m) : scene.type === 'diff' ? diff(chapter,timing,m) : scene.type === 'code' ? code(chapter,timing,m) : flow(scene,timing,m);
  const footer=compact?925:560,source=evidenceFor(chapter,timing.index);
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${m.width} ${m.height}" width="${m.width}" height="${m.height}" role="group" aria-label="${escape(chapter.title)}" font-family="sans-serif"><defs><marker id="arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="5" markerHeight="5" orient="auto-start-reverse"><path d="M0 0 L10 5 L0 10Z" fill="${C.blue}"/></marker></defs><rect width="${m.width}" height="${m.height}" fill="#f7faff"/>${text('SOFTWARE STORIES  /  '+visualForms[scene.type].label.toUpperCase(),m.left,compact?42:35,{size:compact?23:17,color:C.blue,max:55,limit:1,weight:600})}${text(chapter.title,m.left,compact?91:82,{size:compact?36:34,weight:600,max:compact?31:66,limit:compact?2:1})}${compact||chapter.takeaway?'':text(chapter.summary,m.left,116,{size:22,color:C.muted,max:100,limit:1})}${body}${rect(m.left,footer,m.bodyWidth,compact?170:105,C.pale,'#bfd0f1')}${text(`${timing.index+1} / ${scene.steps.length}`,m.left+22,footer+(compact?38:32),{size:compact?25:18,color:C.blue,weight:600})}${text(step.label,m.left+(compact?110:98),footer+(compact?38:33),{size:m.font,weight:600,max:compact?28:80,limit:1})}${chapter.takeaway?(timing.index===scene.steps.length-1?text(chapter.takeaway,m.left+22,footer+(compact?100:76),{size:m.small,color:C.blue,max:compact?40:100,limit:2}):''):text(step.detail,m.left+22,footer+(compact?87:72),{size:m.small,color:C.muted,max:compact?42:100,limit:2})}${text('SOURCE  '+source.path+':'+source.startLine,m.left,m.height-23,{size:compact?21:17,color:C.muted,max:compact?51:115,limit:1})}<rect x="${m.left}" y="${m.height-8}" width="${m.bodyWidth}" height="4" fill="#d4deed"/><rect x="${m.left}" y="${m.height-8}" width="${Math.max(1,clamp(progress)*m.bodyWidth)}" height="4" fill="${C.blue}"/></svg>`;
}
