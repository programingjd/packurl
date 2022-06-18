import {EditorState,Compartment} from 'https://jspm.dev/@codemirror/state';
import {EditorView,keymap,highlightActiveLineGutter} from 'https://jspm.dev/@codemirror/view';
import {history} from 'https://jspm.dev/@codemirror/commands';
import {search,SearchQuery,setSearchQuery,getSearchQuery,findNext,findPrevious,selectMatches,openSearchPanel} from 'https://jspm.dev/@codemirror/search';
import {lintGutter} from 'https://jspm.dev/@codemirror/lint';
import {foldGutter,foldKeymap,codeFolding,bracketMatching} from 'https://jspm.dev/@codemirror/language';
import {closeBrackets} from 'https://jspm.dev/@codemirror/autocomplete';
import {indentWithTab,standardKeymap} from 'https://jspm.dev/@codemirror/commands';
import {indentOnInput} from 'https://jspm.dev/@codemirror/language';
import {darcula} from "./darcula.js";

const tabSize=new Compartment();
const lineWrapping=new Compartment();
const bracketClosing=new Compartment();
const baseTheme=EditorView.baseTheme(
  {
    '.cm-content, .cm-gutter':{minHeight:'100%',font:'1em ccpl,serif',fontDisplay:'block'},
    '.cm-scroller':{overflow:'auto'},
    '.cm-activeLineGutter':{fontWeight:'bold'},
  }
);

const extensions=[
  keymap.of([...standardKeymap,...foldKeymap,/*...commentKeymap,*/indentWithTab]),
  history(),
  darcula,
  baseTheme,
  tabSize.of(EditorState.tabSize.of(2)),
  lineWrapping.of(EditorView.lineWrapping),
  highlightActiveLineGutter(),
  codeFolding({placeholderText:'...'}),
  foldGutter({openText:'-',closedText:'+'}),
  lintGutter(),
  bracketMatching(),
  bracketClosing.of(closeBrackets()),
  indentOnInput(),
];
const state=EditorState.create({extensions});

/** @return {EditorView} */
const installOn=parent=>{
  const view=new EditorView({state,parent});
  return view;
};
export default installOn;
