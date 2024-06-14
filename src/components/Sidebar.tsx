function Sidebar() {
    return <div className="overflow-hidden bg-gray-100" style={{ gridArea: 'side' }}>
        <ul className="ml-0 flex-col flex">
            <li>
                <div className="block relative group/item px-1.5 pb-0.5">
                    <button className="w-full flex gap-1.5 items-center text-sm h-xs px-1.5 rounded-md transition-colors text-gray-600 group-hover/item:text-gray-800 active:bg-highlightSecondary">
                        <div className="flex items-end gap-2 min-w-0">
                            <span className="text-2xs font-mono opacity-50">GET</span>
                            <span className="truncate">https://api.myip.com</span>
                        </div>
                        <div className="ml-auto">
                            <span className="text-2xs dark:opacity-80 font-mono text-green-600">200</span>
                        </div>
                    </button>
                </div>
            </li>
        </ul>
    </div>
}

export default Sidebar;
