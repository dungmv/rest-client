import { useState } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/tauri";
import './App.css';

function App() {
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");
  const [width, setWidth] = useState(150);

  async function greet() {
    // Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
    setGreetMsg(await invoke("greet", { name }));
  }

  const [sidebarWidth, setSidebarWidth] = useState(200);
  const [isSidebarResizing, setIsSidebarResizing] = useState(false);

  const handleMouseDown = () => setIsSidebarResizing(true);

  const handleMouseMove = (e) => {
    if (isSidebarResizing) {
      setSidebarWidth(e.clientX);
    }
  };

  const handleMouseUp = () => {
    setIsSidebarResizing(false);
  };

  return (<div className="grid w-full h-full"
    onMouseMove={handleMouseMove}
    onMouseUp={handleMouseUp}
    style={{
      gridTemplateRows: 'auto minmax(0px, 1fr)',
      gridTemplateColumns: `${sidebarWidth}px 0px 1fr`,
      gridTemplateAreas: `"head head head" "side drag body"`
    }}
  >
    <div className="overflow-hidden bg-gray-100" style={{
      gridRowStart: 'side',
      gridColumnStart: 'side',
      gridRowEnd: 'side',
      gridColumnEnd: 'side'
    }}>Sidebar</div>
    <div className="-translate-x-3 group z-10 flex h-full w-3 cursor-col-resize justify-end right-0"
      onMouseDown={handleMouseDown}
      style={{ gridArea: 'drag' }} />
    <div className="h-md pt-[1px] w-full border-b min-w-0 pl-20 pr-1" style={{
      gridRowStart: 'head',
      gridColumnStart: 'head',
      gridRowEnd: 'head',
      gridColumnEnd: 'head'
    }}>Header</div>
    <div className="p-3 gap-1.5 grid w-full h-full" style={{
      gridRowStart: 'body',
      gridColumnStart: 'body',
      gridRowEnd: 'body',
      gridColumnEnd: 'body',
      gridTemplateRows: 'minmax(0px, 1fr)',
      gridTemplateColumns: `${sidebarWidth}px 0px 1fr`,
      gridTemplateAreas: `"left drag right"`
    }}>
      <div className="h-full grid grid-rows-[auto_minmax(0,1fr)] grid-cols-1 bg-slate-300" style={{
        gridRowStart: 'left',
        gridColumnStart: 'left',
        gridRowEnd: 'left',
        gridColumnEnd: 'left'
      }}>
        Request
      </div>
      <div className="-translate-x-1.5 group z-10 flex h-full w-3 cursor-col-resize justify-center left-0" style={{
        gridRowStart: 'drag',
        gridColumnStart: 'drag',
        gridRowEnd: 'drag',
        gridColumnEnd: 'drag'
      }} />
      <div className="h-full grid grid-rows-[auto_minmax(0,1fr)] grid-cols-1 bg-blue-200" style={{
        gridRowStart: 'right',
        gridColumnStart: 'right',
        gridRowEnd: 'right',
        gridColumnEnd: 'right'
      }}>
        Response
      </div>
    </div>
  </div>);
}

export default App;
