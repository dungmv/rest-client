import { Menu, MenuButton, MenuItem, MenuItems } from "@headlessui/react";
import { PencilIcon } from "@heroicons/react/24/outline";
import { Cog6ToothIcon, ChevronRightIcon } from "@heroicons/react/24/outline";

type TitlebarProps = {
  title: string;
};

function Titlebar({ title }: TitlebarProps) {
  return (
    <div
      className="h-md pt-[1px] h-8 w-full border-b min-w-0 pl-20 pr-1 flex items-center justify-between bg-white"
      style={{ gridArea: "head" }}
      data-tauri-drag-region
    >
      <div className="flex items-center justify-center">
        <Menu>
          <MenuButton>workspace</MenuButton>
          <MenuItems
            anchor="bottom"
            className="rounded border border-white bg-white shadow p-4 mt-1"
          >
            <MenuItem>
              <button className="block data-[focus]:bg-blue-100">
                Settings
              </button>
            </MenuItem>
            <MenuItem>
              <button className="block data-[focus]:bg-blue-100">
                Support
              </button>
            </MenuItem>
            <MenuItem>
              <button className="block data-[focus]:bg-blue-100">
                New Workspace
              </button>
            </MenuItem>
          </MenuItems>
        </Menu>
        <ChevronRightIcon className="w-4 h-4" />
        <Menu>
          <MenuButton>env</MenuButton>
          <MenuItems
            anchor="bottom"
            className="shadow-md rounded border border-white bg-white p-3 text-sm"
          >
            <MenuItem>
              <button className="block data-[focus]:bg-blue-100">dev</button>
            </MenuItem>
            <MenuItem>
              <button className="block data-[focus]:bg-blue-100">prod</button>
            </MenuItem>
            <MenuItem>
              <button className="block data-[focus]:bg-blue-100">local</button>
            </MenuItem>
            <div className="my-1 h-px bg-white" />
            <MenuItem>
              <button className="group flex w-full items-center gap-2 rounded-lg py-1.5 data-[focus]:bg-white">
                <PencilIcon className="size-4 fill-white" />
                Edit
              </button>
            </MenuItem>
          </MenuItems>
        </Menu>
      </div>
      <div className="flex items-center justify-center text-sm">{title}</div>
      <div className="flex items-center justify-center">
        <button className="text-gray-600 w-8 h-8">
          <Cog6ToothIcon className="p-2" />
        </button>
      </div>
    </div>
  );
}

export default Titlebar;
