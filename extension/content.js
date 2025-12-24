/* Simple content script to enable comment anchoring and show live notifications */
(function(){
  let commentMode = false;
  let lastHover = null;

  function computeSelector(el){
    if(!el) return null;
    let path = [];
    while(el && el.nodeType === Node.ELEMENT_NODE){
      let selector = el.nodeName.toLowerCase();
      if(el.id){
        selector += '#'+el.id;
        path.unshift(selector);
        break;
      } else {
        let sib = el, nth = 1;
        while((sib = sib.previousElementSibling) != null){
          if(sib.nodeName === el.nodeName) nth++;
        }
        if(nth !== 1){ selector += `:nth-of-type(${nth})`; }
      }
      path.unshift(selector);
      el = el.parentElement;
    }
    return path.join(' > ');
  }

  function highlight(el){
    if(!el) return;
    el.style.outline = '3px solid #ffcc00';
  }
  function clearHighlight(el){
    if(!el) return;
    el.style.outline = '';
  }

  function onClick(e){
    if(!commentMode) return;
    e.preventDefault();
    e.stopPropagation();
    const selector = computeSelector(e.target);
    const payload = {selector, href: location.href};
    // send to extension popup
    window.postMessage({type:'cospace-comment-anchor', payload}, '*');
    disableMode();
  }

  function onMouseOver(e){
    if(!commentMode) return;
    if(lastHover && lastHover !== e.target) clearHighlight(lastHover);
    highlight(e.target);
    lastHover = e.target;
  }

  function enableMode(){
    commentMode = true;
    document.addEventListener('click', onClick, true);
    document.addEventListener('mouseover', onMouseOver, true);
  }
  function disableMode(){
    commentMode = false;
    document.removeEventListener('click', onClick, true);
    document.removeEventListener('mouseover', onMouseOver, true);
    if(lastHover) clearHighlight(lastHover);
    lastHover = null;
  }

  // listen to messages from extension popup
  window.addEventListener('message', (e)=>{
    if(e.source !== window) return;
    const data = e.data || {};
    if(data.type === 'cospace-enable-comment-mode'){
      enableMode();
    } else if(data.type === 'cospace-disable-comment-mode'){
      disableMode();
    }
  });

  // NEW: Long-poll loop for comments for active cospace
  let longPollAbort = null;
  let lastSeenId = 0;

  async function getActiveCospaceId(){
    return new Promise((resolve)=>{
      try{ chrome.storage.local.get(['activeCospaceId'], (res)=>{ resolve(res.activeCospaceId); }); } catch(e){ resolve(null); }
    });
  }

  function showToast(text, onClick){
    const t = document.createElement('div'); t.className = 'toast'; t.textContent = text; document.body.appendChild(t);
    t.addEventListener('click', ()=>{ if(onClick) onClick(); t.remove(); });
    setTimeout(()=>{ t.remove(); }, 8000);
  }

  function attachBadgeToElement(el, comment){
    if(!el) return;
    const previousOutline = el.style.outline;
    el.style.outline = '3px solid #4ade80';
    const badge = document.createElement('div'); badge.textContent = '💬'; badge.className = 'cospace-badge';
    Object.assign(badge.style, { position:'absolute', background:'rgba(0,0,0,0.7)', color:'#fff', padding:'2px 6px', borderRadius:'12px', fontSize:'12px', zIndex:999999 });
    document.body.appendChild(badge);
    badge.addEventListener('click', ()=>{ alert(comment.author + ': ' + comment.text); });
    // track badge for updates
    window.__cospace_badges = window.__cospace_badges || [];
    const entry = { el, badge, comment, previousOutline };
    window.__cospace_badges.push(entry);
    updateBadgePositions();
    // remove after some time
    setTimeout(()=>{ try{ badge.remove(); el.style.outline = previousOutline; window.__cospace_badges = (window.__cospace_badges || []).filter(b => b.badge !== badge); } catch(e){} }, 15000);
  }

  function updateBadgePositions(){
    const items = window.__cospace_badges || [];
    // sort by element top to avoid overlap stacking
    items.sort((a,b)=>{ const ra = a.el.getBoundingClientRect(); const rb = b.el.getBoundingClientRect(); return ra.top - rb.top; });
    for(let i=0;i<items.length;i++){
      const {el, badge} = items[i];
      try{
        const rect = el.getBoundingClientRect();
        badge.style.left = (window.scrollX + rect.right - 18) + 'px';
        badge.style.top = (window.scrollY + rect.top - 8 + i*18) + 'px';
      } catch(e){ /* element removed */ badge.remove(); }
    }
  }

  let updateTimer = null;
  function scheduleUpdate(){ if(updateTimer) return; updateTimer = setTimeout(()=>{ updateTimer = null; updateBadgePositions(); }, 50); }
  window.addEventListener('scroll', scheduleUpdate, true);
  window.addEventListener('resize', scheduleUpdate, true);
  // Mutation observer to catch element moves/removals
  const mo = new MutationObserver(scheduleUpdate);
  mo.observe(document.body, { attributes:true, childList:true, subtree:true });

  async function longPollLoop(){
    // abort previous loop if any
    if(longPollAbort){ try{ longPollAbort.abort(); } catch(e){} }
    longPollAbort = new AbortController();
    const signal = longPollAbort.signal;
    const cospaceId = await getActiveCospaceId();
    if(!cospaceId) return;
    while(true){
      try{
        const res = await fetch('http://localhost:5000/api/comments/longpoll/' + cospaceId + '?since_id=' + lastSeenId + '&timeout=25', {credentials:'include', signal});
        if(!res.ok) break;
        const items = await res.json();
        if(items && items.length){
          for(const c of items){
            if(c.id > lastSeenId) lastSeenId = c.id;
            showToast((c.author||'Anonymous') + ': ' + (c.text.length>120? c.text.slice(0,120)+'...': c.text), ()=>{ window.open(chrome.runtime.getURL('dashboard.html'), '_blank'); });
            if(c.selector){
              try{ const el = document.querySelector(c.selector); if(el) attachBadgeToElement(el, c); } catch(e){}
            }
          }
        }
        // continue loop immediately for next batch
      } catch(err){
        // network or abort -> break if aborted, otherwise retry after a short wait
        if(err.name === 'AbortError') break;
        console.error('longpoll error', err);
        await new Promise(r=>setTimeout(r, 2000));
      }
    }
  }

  async function startLongPoll(){
    await longPollLoop();
  }
  async function stopLongPoll(){ if(longPollAbort){ try{ longPollAbort.abort(); } catch(e){} longPollAbort = null; } }

  // Start/stop longpoll based on active cospace
  getActiveCospaceId().then((id)=>{ if(id) startLongPoll(); });
  chrome.storage.onChanged.addListener((changes, area)=>{
    if(area === 'local' && changes.activeCospaceId){
      const v = changes.activeCospaceId.newValue;
      if(v){ lastSeenId = 0; startLongPoll(); } else stopLongPoll();
    }
  });

})();
