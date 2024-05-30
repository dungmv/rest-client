import { Cog6ToothIcon } from "@heroicons/react/24/outline";

function Titlebar({ title }) {
    return <div className="h-md pt-[1px] h-8 w-full border-b min-w-0 pl-20 pr-1 flex items-center justify-between bg-white" style={{ gridArea: 'head' }} data-tauri-drag-region>
        <div>emddi</div>
        <div>{title}</div>
        <div>
            <button className='text-gray-600 w-8 h-8'><Cog6ToothIcon className="p-2" /></button>
        </div>
    </div>
}

export default Titlebar;
