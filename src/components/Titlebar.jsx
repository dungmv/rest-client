import { Menu, MenuButton, MenuItem, MenuItems } from '@headlessui/react'
import { Cog6ToothIcon, PlusCircleIcon, ChevronRightIcon } from "@heroicons/react/24/outline";

function Titlebar({ title }) {
    return <div className="h-md pt-[1px] h-8 w-full border-b min-w-0 pl-20 pr-1 flex items-center justify-between bg-white" style={{ gridArea: 'head' }} data-tauri-drag-region>
        <div className='flex items-center justify-center'>
            <button type="button" className='text-gray-600 w-8 h-8'><PlusCircleIcon className="p-2" /></button>
            <Menu>
                <MenuButton>workspace</MenuButton>
                <MenuItems anchor="bottom" className='rounded-xl border border-white/5 bg-white/5 p-1 text-sm/6'>
                    <MenuItem>
                        <a className="block data-[focus]:bg-blue-100" href="/settings">
                            Settings
                        </a>
                    </MenuItem>
                    <MenuItem>
                        <a className="block data-[focus]:bg-blue-100" href="/support">
                            Support
                        </a>
                    </MenuItem>
                    <MenuItem>
                        <a className="block data-[focus]:bg-blue-100" href="/license">
                            License
                        </a>
                    </MenuItem>
                </MenuItems>
            </Menu>
            <ChevronRightIcon className="w-4 h-4" />
            <Menu>
                <MenuButton>env</MenuButton>
                <MenuItems anchor="bottom" className='rounded-xl border border-white/5 bg-white/5 p-1 text-sm/6'>
                    <MenuItem>
                        <a className="block data-[focus]:bg-blue-100" href="/settings">
                            Settings
                        </a>
                    </MenuItem>
                    <MenuItem>
                        <a className="block data-[focus]:bg-blue-100" href="/support">
                            Support
                        </a>
                    </MenuItem>
                    <MenuItem>
                        <a className="block data-[focus]:bg-blue-100" href="/license">
                            License
                        </a>
                    </MenuItem>
                </MenuItems>
            </Menu>
        </div>
        <div className='flex items-center justify-center text-sm'>{title}</div>
        <div className='flex items-center justify-center'>
            <button className='text-gray-600 w-8 h-8'><Cog6ToothIcon className="p-2" /></button>
        </div>
    </div>
}

export default Titlebar;
