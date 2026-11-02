// Official website only: original login, playlists, search and player.
(() => {
 if(location.origin!=='https://music.pentasys.id.vn')return;
 const boot=()=>{
  // Use the website's own controls and session. Report success only after checking the result.
  window.pentakuruMusicCommand=(command)=>{
   if(window.pentakuruMusicStatus?.done===false&&!window.pentakuruMusicStatus.cancelled)return window.pentakuruMusicStatus.request_id===command.request_id?{started:true}:null;
   const status=window.pentakuruMusicStatus={started:true,done:false,ok:false,request_id:command.request_id};
   const finish=(ok,message)=>Object.assign(status,{done:true,ok,message});
   const wait=async(predicate)=>{
    const deadline=Date.now()+25000;
    while(Date.now()<deadline){if(status.cancelled)throw Error('Đã dừng lệnh Music.');const result=predicate();if(result)return result;await new Promise(resolve=>setTimeout(resolve,150));}
    throw Error('Penta Music chưa có kết quả. Hãy kiểm tra khung nhạc, kết nối hoặc chọn bài rồi bấm Play.');
   };
   (async()=>{
    try{
     await wait(()=>document.getElementById('audio-engine'));
     const engines=()=>['audio-engine','audio-engine-b'].map(id=>document.getElementById(id)).filter(Boolean);
     const playing=()=>engines().some(audio=>!audio.paused);
     if(command.action==='pause'){
      if(playing()){
       const control=document.getElementById('btn-play-pause');
       if(!control)throw Error('Chưa tìm thấy nút tạm dừng Penta Music.');
       control.click();
      }
      await wait(()=>!playing());finish(true,'Đã tạm dừng Penta Music.');return;
     }
     let target;
     if(command.action==='search'||command.action==='play_search'){
      const input=await wait(()=>document.getElementById('topbar-search-input'));
      input.value=command.query;input.focus();
      input.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',code:'Enter',bubbles:true,cancelable:true}));
      target=await wait(()=>{
       const cards=Array.from(document.querySelectorAll('#tracks-grid .track-card'));
       const heading=document.getElementById('result-heading')?.textContent||'';
       const refreshed=heading.includes('Kết Quả Tìm Kiếm:')&&heading.includes(command.query);
       return refreshed&&cards[0];
      });
      if(command.action==='search'){finish(true,'Đã tìm bài «'+command.query+'» trong Penta Music. Anh chọn bài trong danh sách kết quả.');return;}
     }else if(command.action!=='play'){throw Error('Lệnh Music không được hỗ trợ.');}
     if(target){target.click();}
     else if(!playing()){
      const control=engines().some(audio=>audio.currentSrc||audio.getAttribute('src'))?document.getElementById('btn-play-pause'):document.querySelector('#tracks-grid .track-card');
      if(!control)throw Error('Chưa có bài để phát. Anh hãy nhắn “tìm bài hát + tên bài” hoặc chọn một bài trong Penta Music.');
      control.click();
     }
     await wait(()=>engines().some(audio=>!audio.paused&&audio.readyState>=2));
     finish(true,'Penta Music đang phát: '+(document.getElementById('player-title')?.textContent?.trim()||'bài hiện tại')+'.');
    }catch(error){finish(false,error.message||String(error));}
   })();
   return {started:true};
  };
  document.documentElement.dataset.pentakuruWeb='true';
  const style=document.createElement('style');
  style.textContent=`:root{--bg-body:#171719!important;--bg-surface:#202023!important}body{background:#171719!important;overflow-x:hidden!important}
  @media(max-width:620px){.app-layout{display:block!important;min-width:0!important}.sidebar-rail,.topbar-logo{display:none!important}.main-area{margin-left:0!important;min-width:0!important;width:100%!important}.main-topbar{height:auto!important;flex-wrap:wrap!important;padding:8px!important;gap:6px!important}.topbar-left{min-width:0!important;flex:1!important}.topbar-search-container{width:100%!important;min-width:0!important;margin:0!important}#topbar-search-input{min-width:0!important;width:100%!important}.workspace-scroll{padding:8px!important;padding-bottom:150px!important}#tracks-grid{grid-template-columns:1fr!important;gap:8px!important}#player-bar{max-width:calc(100vw - 12px)!important;min-width:0!important;left:6px!important;right:6px!important;transform:none!important}#player-bar .mini-player-top-row,#player-bar .mini-controls{flex-wrap:wrap!important;gap:6px!important}.modal-card,.auth-modal-card{max-width:calc(100vw - 16px)!important;max-height:90vh!important;overflow:auto!important}}`;
  document.head.appendChild(style);
  const search=document.querySelector('.topbar-search-container');
  if(search){const ensure=()=>{if(search.style.getPropertyValue('display')!=='flex')search.style.setProperty('display','flex','important');};ensure();new MutationObserver(ensure).observe(search,{attributes:true,attributeFilter:['style']});}
 };
 if(document.readyState==='loading')document.addEventListener('DOMContentLoaded',boot,{once:true});else boot();
})();
