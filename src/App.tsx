import React, { useEffect, useRef, useState } from "react";
import { window as tauriWindow } from "@tauri-apps/api";
import { fetch } from '@tauri-apps/plugin-http';
import { BaseDirectory, readDir } from "@tauri-apps/plugin-fs";
import { Menu, MenuButton, MenuItem, MenuItems } from '@headlessui/react'
import { PaperAirplaneIcon } from "@heroicons/react/24/outline";
import { json } from "@codemirror/lang-json"
import { lineNumbers, highlightSpecialChars, } from "@codemirror/view"
import { EditorState, } from "@codemirror/state"
import { basicSetup, EditorView } from "codemirror"
import Sidebar from './components/Sidebar';
import Titlebar from './components/Titlebar';

function App() {
  const editorViewRef = useRef<EditorView>(null);
  const urlInputRef = useRef<HTMLInputElement>(null);
  const contentRef = useRef<HTMLDivElement>(null);
  const responseRef = useRef<HTMLDivElement>(null);

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

  useEffect(() => {
    console.log('Reading directory...')
    readDir('nexgen', {baseDir: BaseDirectory.Document}).then(console.log).catch(console.error);
  }, [])

  const [requestWidth, setRequestWidth] = useState(0.5);
  const [sidebarWidth, setSidebarWidth] = useState(200);

  const resizeRequest = (e: React.DragEvent<HTMLDivElement>) => {
    if (!contentRef.current) return;
    const { width, left } = contentRef.current.getBoundingClientRect()
    const fr = (e.clientX - left) / width;
    setRequestWidth(fr);
  }

  const resizeSidebar = (e: React.DragEvent<HTMLDivElement>) => {
    setSidebarWidth(e.clientX);
  };

  const sendRequest = async () => {
    if (!urlInputRef.current || !editorViewRef.current) return;
    const response = await fetch(urlInputRef.current.value)
    const value = await response.json()
    const transaction = editorViewRef.current.state.update({
      changes: { from: 0, to: editorViewRef.current.state.doc.length, insert: JSON.stringify(value, null, 2) }
    });
    editorViewRef.current.dispatch(transaction);
  }

  return (<div className="grid w-full h-full text-gray-600"
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
        <div className="relative rounded-md shadow-sm">
          <div className='absolute inset-y-0 left-0 flex items-center'>
            <Menu>
                <MenuButton className='pl-2'>GET</MenuButton>
                <MenuItems anchor="top start" className='pl-2 rounded-xl border border-white/5 bg-white/5 p-1 text-sm/6'>
                    <MenuItem>
                        <button className="block data-[focus]:bg-blue-100">
                            GET
                        </button>
                    </MenuItem>
                    <MenuItem>
                        <button className="block data-[focus]:bg-blue-100">
                            POST
                        </button>
                    </MenuItem>
                    <MenuItem>
                        <button className="block data-[focus]:bg-blue-100">
                            PUT
                        </button>
                    </MenuItem>
                </MenuItems>
            </Menu>
          </div>
          <input
            ref={urlInputRef}
            type="text"
            defaultValue={"https://api.myip.com"}
            className="block w-full rounded-md border-0 pl-12 pr-10 ring-1 ring-inset ring-gray-300 placeholder:text-gray-400 focus:ring-2 focus:ring-inset focus:ring-indigo-600 "
            placeholder="https://api.restclient.com"
          />
          <div className="absolute inset-y-0 right-0 flex items-center">
            <button className="w-10 h-10" onClick={sendRequest}>
              <PaperAirplaneIcon className="p-2" />
            </button>
          </div>
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
