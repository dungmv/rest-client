import './App.css';
import React, { useEffect, useRef, useState } from "react";
import { window as tauriWindow } from "@tauri-apps/api";
import { invoke } from "@tauri-apps/api/core";
import { fetch } from '@tauri-apps/plugin-http';
import Database from "@tauri-apps/plugin-sql";
import { PaperAirplaneIcon } from "@heroicons/react/24/outline";
import { json } from "@codemirror/lang-json"
import { lineNumbers, highlightSpecialChars, } from "@codemirror/view"
import { EditorState, } from "@codemirror/state"
import { basicSetup, EditorView } from "codemirror"
import Sidebar from './components/Sidebar';
import Titlebar from './components/Titlebar';
import useDatabase from './hooks/useDatabase';

function App() {
  /** @type {React.MutableRefObject<Database>} */
  const dbRef = useRef();
  /** @type {React.MutableRefObject<EditorView>} */
  const editorViewRef = useRef();
  /** @type {React.MutableRefObject<HTMLInputElement>} */
  const urlInputRef = useRef();
  /** @type {React.MutableRefObject<HTMLDivElement>} */
  const contentRef = useRef();
  const responseRef = useRef();
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");
  const db = useDatabase()

  async function greet() {
    // Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
    setGreetMsg(await invoke("greet", { name }));
  }

  const [title, setTitle] = useState("");
  const win = tauriWindow.getCurrent();

  useEffect(() => {
    win.title().then(setTitle);
  }, [win]);

  useEffect(() => {
    if (responseRef.current && !editorViewRef.current) {
      editorViewRef.current = new EditorView({
        doc: "{}",
        extensions: [json(), lineNumbers(), highlightSpecialChars(), EditorState.readOnly.of(true), basicSetup],
        parent: responseRef.current
      })
    }
  }, [responseRef.current]);


  const [requestWidth, setRequestWidth] = useState(0.5);
  const [sidebarWidth, setSidebarWidth] = useState(200);

  const resizeRequest = (e) => {
    const { width, left } = contentRef.current.getBoundingClientRect()
    const fr = (e.clientX - left) / width;
    setRequestWidth(fr);
  }

  const resizeSidebar = (e) => {
    setSidebarWidth(e.clientX);
  };

  const sendRequest = async () => {
    const response = await fetch(urlInputRef.current.value)
    const value = await response.json()
    const transaction = editorViewRef.current.state.update({
      changes: { from: 0, to: editorViewRef.current.state.doc.length, insert: JSON.stringify(value, null, 2) }
    });
    editorViewRef.current.dispatch(transaction);

    // db.execute('INSERT INTO requests (url, response) VALUES (?, ?)', [urlInputRef.current.value, JSON.stringify(response.data, null, 2)])
    // console.log('Inserted')
  }

  return (<div className="grid w-full h-full"
    style={{
      gridTemplateRows: 'auto minmax(0px, 1fr)',
      gridTemplateColumns: `${sidebarWidth}px 0px 1fr`,
      gridTemplateAreas: `"head head head" "side drag body"`
    }}
  >
    <Sidebar />
    <div className="-translate-x-3 group z-10 flex h-full w-3 cursor-col-resize justify-end right-0"
      draggable onDrag={resizeSidebar}
      style={{ gridArea: 'drag' }} />
    <Titlebar title={title} />
    <div ref={contentRef} className="p-3 gap-1.5 grid w-full h-full" style={{
      gridArea: 'body',
      gridTemplateRows: 'minmax(0px, 1fr)',
      gridTemplateColumns: `${requestWidth}fr 0px ${1 - requestWidth}fr`,
      gridTemplateAreas: `"left drag right"`
    }}>
      <div className="h-full grid grid-rows-[auto_minmax(0,1fr)] grid-cols-1 bg-white" style={{ gridArea: 'left' }}>
        <div className="flex items-center border border-gray-300 rounded-lg w-full">
          <button className="text-gray-600 w-16 h-10">
            POST
          </button>
          <input
            ref={urlInputRef}
            type="text"
            defaultValue={"https://api.myip.com"}
            className="flex-auto px-0 py-2 border-none focus:outline-none h-10"
            placeholder="url"
          />
          <button className="text-gray-600 w-10 h-10" onClick={sendRequest}>
            <PaperAirplaneIcon className="p-2" />
          </button>
        </div>
      </div>
      <div draggable={true} className="-translate-x-1.5 group z-10 flex h-full w-3 cursor-col-resize justify-center left-0"
        onDrag={resizeRequest}
        style={{ gridArea: 'drag' }}
      />
      <div className="h-full grid grid-rows-[auto_minmax(0,1fr)] grid-cols-1 p-2 border border-gray-300 rounded-lg bg-white" style={{ gridArea: 'right' }}>
        <div ref={responseRef} className='border border-gray-300 rounded-lg w-full p-2' />
      </div>
    </div>
  </div>);
}

export default App;
