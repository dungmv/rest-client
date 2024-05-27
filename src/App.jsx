import './App.css';
import React, { useEffect, useRef, useState } from "react";
import { window as tauriWindow } from "@tauri-apps/api";
import { invoke } from "@tauri-apps/api/tauri";
import { PaperAirplaneIcon } from "@heroicons/react/24/outline";
import { fetch } from '@tauri-apps/api/http';

function App() {
  const urlInputRef = useRef();
  const contentRef = useRef();
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");
  const [response, setResponse] = useState("");

  async function greet() {
    // Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
    setGreetMsg(await invoke("greet", { name }));
  }

  const [title, setTitle] = useState("");
  const win = tauriWindow.getCurrent();

  useEffect(() => {
    win.title().then(setTitle);
  }, [win]);

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
    fetch(urlInputRef.current.value)
      .then(resp => setResponse(resp.data))
  }

  return (<div className="grid w-full h-full"
    style={{
      gridTemplateRows: 'auto minmax(0px, 1fr)',
      gridTemplateColumns: `${sidebarWidth}px 0px 1fr`,
      gridTemplateAreas: `"head head head" "side drag body"`
    }}
  >
    <div className="overflow-hidden bg-gray-100" style={{ gridArea: 'side' }}>Sidebar</div>
    <div className="-translate-x-3 group z-10 flex h-full w-3 cursor-col-resize justify-end right-0"
      draggable={true} onDrag={resizeSidebar}
      style={{ gridArea: 'drag' }} />
    <div className="h-md pt-[1px] h-8 w-full border-b min-w-0 pl-20 pr-1" style={{ gridArea: 'head' }} data-tauri-drag-region>
      <span>{title}</span>
    </div>
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
      <div className="h-full grid grid-rows-[auto_minmax(0,1fr)] grid-cols-1 bg-blue-200" style={{ gridArea: 'right' }}>
        {JSON.stringify(response)}
      </div>
    </div>
  </div>);
}

export default App;
