import { PlusCircleIcon } from "@heroicons/react/24/outline";

function Sidebar() {
    return <div className="overflow-hidden bg-gray-100" style={{ gridArea: 'side' }}>
        <div className="flex items-center p-1.5">
            <input type="text" className="h-8 w-full border-gray-300 rounded-md placeholder:text-gray-400" name="search" placeholder="Search" />
            <button type="button" className=' p-1  w-8 h-8'>
                <PlusCircleIcon className="p-0" />
            </button>
        </div>
        <div className="flex-1 overflow-y-auto">
            <ul className="ml-0 flex-col flex">
                <li>
                    <div className="block relative group/item px-1.5 pb-0.5">
                        <button className="w-full flex gap-1.5 items-center text-sm h-xs px-1.5 rounded-md transition-colors group-hover/item:text-gray-800 active:bg-highlightSecondary">
                            <div className="flex items-end gap-2 min-w-0">
                                <span className="text-2xs font-mono opacity-50">GET</span>
                                <span className="truncate">https://api.myip.com</span>
                            </div>
                            <div className="ml-auto">
                                <span className="text-2xs font-mono text-green-600">200</span>
                            </div>
                        </button>
                    </div>
                </li>
            </ul>
        </div>
    </div>
}

export default Sidebar;
