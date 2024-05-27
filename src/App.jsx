import React, { useEffect, useRef, useState } from "react";
import { appWindow } from "@tauri-apps/api/window";
import { window as tauriWindow } from "@tauri-apps/api";
import { invoke } from "@tauri-apps/api/tauri";
import './App.css';

function App() {
  const contentRef = useRef();
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");

  async function greet() {
    // Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
    setGreetMsg(await invoke("greet", { name }));
  }

  const [title, setTitle] = useState("");
  const win = tauriWindow.getCurrent();

  useEffect(() => {
    win.title().then(setTitle);
  }, [win]);

  const [isScaleup, setScaleup] = useState(false);
  // .minimize() - to minimize the window
  const onMinimize = () => appWindow.minimize();
  const onScaleup = () => {
    // .toggleMaximize() - to swap the window between maximize and minimum
    appWindow.toggleMaximize();
    setScaleup(true);
  }

  const onScaledown = () => {
    appWindow.toggleMaximize();
    setScaleup(false);
  }

  // .close() - to close the window
  const onClose = () => appWindow.close();

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
      <div className="h-full grid grid-rows-[auto_minmax(0,1fr)] grid-cols-1 bg-slate-300" style={{ gridArea: 'left' }}>
        <input type="text" className="form-input rounded-md" value={name} onChange={(e) => setName(e.target.value)} />
      </div>
      <div draggable={true} className="-translate-x-1.5 group z-10 flex h-full w-3 cursor-col-resize justify-center left-0"
        onDrag={resizeRequest}
        style={{ gridArea: 'drag' }}
      />
      <div className="h-full grid grid-rows-[auto_minmax(0,1fr)] grid-cols-1 bg-blue-200" style={{ gridArea: 'right' }}>
        Response
      </div>
    </div>
  </div>);
}

export default App;
