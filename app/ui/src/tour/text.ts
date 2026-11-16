// The guided tour's strings (P-020, UI-18): the final English text of design/screens/18-tour.html, in one place.

import { THIS_COMPUTER } from "../shared/platform";

export interface StepText {
  title: string;
  text: string;
  /** A hint when the lit area invites a gesture ("Try it: …"). */
  try?: string;
  /** The closing card's three places to start. */
  list?: [icon: "plus" | "spark" | "play", title: string, text: string][];
}

export const STEP_TEXT: StepText[] = [
  { title: "Welcome to Stemwise", text: `A three-minute tour on a demo song: the mix, the assistant, the master and the export. Nothing you change here is saved, and nothing leaves ${THIS_COMPUTER}.` },
  { title: "Play the song", text: "The top bar holds the transport, the position in the song and the meters of the whole mix. The demo keeps playing while you look around.", try: "press Play, or Space, to listen." },
  { title: "Tracks on a timeline", text: "Each imported file becomes a track, laid out under the sections of the song. Editing never touches your files, and every change can be undone." },
  { title: "A track", text: "The header holds the name, the role, mute, solo and the lock that keeps the assistant away. The small labels tell what is working in its chain." },
  { title: "The mixer", text: "One strip for each track, group and shared effect: level, pan, the sends to the reverb and the delay, and the meters. The grid button shows every setting at once.", try: "move a fader while it plays." },
  { title: "The inspector", text: "The chain of the selected track, in the order the sound goes through it. Open a block to change it, or switch it off to hear the difference." },
  { title: "The assistant", text: "Fill in the brief, then ask for an Auto mix or for a change in your own words. The model never receives audio, only measurements and names, and you see the cost before anything is sent." },
  { title: "A proposal to review", text: "Nothing changes until you decide. Read why each track changes, compare by ear with A/B, then apply all of it, only the tracks you pick, or none.", try: "switch between Current mix and Proposal while it plays." },
  { title: "One step to undo", text: "The tour applied the proposal: it is a single step in the history, and one Undo takes all of it back. Snapshots keep named versions to compare." },
  { title: "Explain this mix", text: "Where the sound goes, drawn as a map, and the same mix told in words, built from the project without any model. Click a name to follow its path." },
  { title: "Mix, then Master", text: "Mix and Master are two phases, each with its own history. Mastering starts from the mixdown, the mix bounced to one stereo file: the tour has made one for you." },
  { title: "The master chain", text: "One chain for the whole song, in the order you choose; the limiter and dither always come last. Click a block to open it beside the rack." },
  { title: "Loudness and A/B", text: "The target decides how loud the file will be, and the meters show where the master stands. A/B in the top bar compares it with the mixdown, at its real level or at the same loudness." },
  { title: "Export", text: "Write WAV, FLAC and AAC in one go, with title, artist, ISRC and cover in every file. In the tour nothing is written: here you only look." },
  {
    title: "Ready for your own songs",
    text: "The demo closes and you are back on the Welcome screen. Three places to start:",
    list: [
      ["plus", "New project", "Drop a folder of stems on the window."],
      ["spark", "Set up the assistant", "Connect your OpenRouter account, or use the offline demo."],
      ["play", "Help › Take the Tour", "Shows this tour again, at any time."],
    ],
  },
];

export const TOUR_TEXT = {
  dialog: "Guided tour",
  note: `The demo song is synthesised on ${THIS_COMPUTER} from simple sounds, so it sounds a little plain.`,
  keys: { next: "next", back: "back", esc: "leaves the tour", space: "plays" },
  start: "Start",
  skip: "Skip tour",
  back: "Back",
  next: "Next",
  done: "Done",
  tryIt: "Try it:",
  waitDemo: ["Preparing the demo song…", "a moment"],
  waitProposal: ["Preparing the proposal…", "a few seconds"],
  waitMixdown: ["Bouncing the mix to the mixdown…", ""],
  waitMaster: ["Preparing the master…", ""],
  failedTitle: "The demo song can’t be prepared",
  failedText: `Something went wrong while building it on ${THIS_COMPUTER}, so the tour ends here. Everything else works as usual.`,
  failedNote: "Help › Take the Tour tries again.",
  copyDetails: "Copy details",
  close: "Close",
  skippedTitle: "Tour skipped",
  skippedText: "Take it again at any time from Help › Take the Tour, or from this screen.",
  saveFirstSub: "The tour opens its own demo song, so this project closes first. If you don’t save, the changes since the last save are lost.",
  status: "Tour · nothing is saved",
  exportOff: "Exporting is off in the tour",
  exportFoot: "In the tour nothing is written",
  welcomeLink: "Take the tour",
  welcomeLength: "· about 3 minutes",
  welcomeTip: "A three-minute tour on a demo song",
  menuItem: "Take the Tour",
  menuHelp: "Help",
  announce: (n: number, of: number, title: string) => `Step ${n} of ${of}: ${title}`,
  counter: (n: number, of: number) => `${n} of ${of}`,
};
