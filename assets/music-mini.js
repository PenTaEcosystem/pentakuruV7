// Official website only: original login, playlists, search and player.
(() => {
 if(location.origin!=='https://music.pentasys.id.vn')return;
 const boot=()=>{
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
