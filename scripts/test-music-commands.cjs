// Run with node scripts/test-music-commands.cjs. No network, account or desktop input.
const { readFileSync } = require('node:fs');
const vm = require('node:vm');
const assert = require('node:assert/strict');
const source = readFileSync('assets/music-mini.js', 'utf8');

function fixture({ paused = true, loaded = true, cards = true, refusePlay = false, secondaryPlaying = false } = {}) {
  let clock = 0, clicks = 0, search = '', focused = false;
  const audio = { paused, readyState: 3, currentSrc: loaded ? 'song.mp3' : '',
    getAttribute: () => '', pause() { this.paused = true; } };
  const card = () => ({ click() { clicks++; if (!refusePlay) audio.paused = false; } });
  let results = cards ? [card()] : [];
  const heading = { textContent: '' };
  const input = { value: '', focus() { focused = true; }, dispatchEvent(event) {
    assert.equal(event.key, 'Enter'); search = this.value;
    // Old results remain briefly: the bridge must wait for fresh nodes.
    heading.textContent = 'Đang tìm kiếm';
    Promise.resolve().then(() => { results = [card()]; heading.textContent = 'Kết Quả Tìm Kiếm: "'+search+'"'; });
  } };
  const elements = { 'audio-engine': audio, 'audio-engine-b': secondaryPlaying ? {paused:false,readyState:3} : null, 'topbar-search-input': input, 'result-heading': heading,
    'btn-play-pause': { click() { clicks++; if (!refusePlay) audio.paused = !audio.paused; } },
    'player-title': { textContent: 'Nàng Thơ' } };
  const window = {};
  const context = { window, location: { origin: 'https://music.pentasys.id.vn' },
    document: { readyState: 'complete', documentElement: { dataset: {} },
      head: { appendChild() {} }, createElement: () => ({}),
      getElementById: id => elements[id],
      querySelector: selector => selector.includes('track-card') ? results[0] : null,
      querySelectorAll: () => results },
    Date: { now: () => clock }, KeyboardEvent: function (_, options) { return options; },
    setTimeout(callback) { clock += 1000; Promise.resolve().then(callback); } };
  vm.runInNewContext(source, context);
  return { window, audio, get clicks() { return clicks; }, get search() { return search; },
    get focused() { return focused; }, async run(action, query) {
      window.pentakuruMusicCommand({ action, query, request_id: 'test' });
      for (let i = 0; i < 200 && !window.pentakuruMusicStatus.done; i++) await Promise.resolve();
      assert.equal(window.pentakuruMusicStatus.done, true);
      return window.pentakuruMusicStatus;
    } };
}

(async () => {
  const resume = fixture(); assert.equal((await resume.run('play')).ok, true); assert.equal(resume.clicks, 1);
  const playing = fixture({ paused: false }); await playing.run('play'); assert.equal(playing.clicks, 0);
  const crossfade = fixture({ secondaryPlaying: true }); assert.equal((await crossfade.run('play')).ok, true); assert.equal(crossfade.clicks, 0);
  const initial = fixture({ loaded: false }); assert.equal((await initial.run('play')).ok, true);
  const empty = fixture({ loaded: false, cards: false }); assert.equal((await empty.run('play')).ok, false);
  const search = fixture(); assert.equal((await search.run('search', 'Nàng Thơ')).ok, true);
  assert.equal(search.search, 'Nàng Thơ'); assert.equal(search.focused, true); assert.equal(search.clicks, 0);
  const playSearch = fixture(); assert.equal((await playSearch.run('play_search', 'Nàng Thơ')).ok, true); assert.equal(playSearch.clicks, 1);
  const pause = fixture({ paused: false }); assert.equal((await pause.run('pause')).ok, true); assert.equal(pause.audio.paused, true);
  const blocked = fixture({ refusePlay: true }); assert.equal((await blocked.run('play')).ok, false);
  const cancelled = fixture(); cancelled.window.pentakuruMusicCommand({action:'search',query:'Test',request_id:'cancel'});
  cancelled.window.pentakuruMusicStatus.cancelled = true;
  for(let i=0;i<50;i++) await Promise.resolve();
  assert.equal(cancelled.window.pentakuruMusicStatus.ok, false); assert.equal(cancelled.clicks, 0);
  console.log('Music bridge: 10 scenarios passed.');
})().catch(error => { console.error(error); process.exitCode = 1; });
