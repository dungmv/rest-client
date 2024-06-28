import { Menu, MenuButton, MenuItem, MenuItems } from "@headlessui/react";
import { PencilIcon } from "@heroicons/react/24/outline";
import { Cog6ToothIcon, ChevronRightIcon } from "@heroicons/react/24/outline";
import { writeTextFile, BaseDirectory, readDir } from "@tauri-apps/plugin-fs";
import { open } from '@tauri-apps/plugin-dialog';
import YAML from 'yaml'
import { useEffect, useState } from "react";

type TitlebarProps = {
  title: string;
};

function Titlebar({ title }: TitlebarProps) {
  const [wd, setWD] = useState('')

  const createWorkspace = () => {
    const yml = YAML.stringify({
      version: '1',
      name: 'get ip',
      url: 'https://api.myip.com',
      method: 'GET',
      headers: [
        'Accept: application/json',
        `Content-Type: application/json`,
      ],
      body: JSON.stringify({ ip: '127.0.0.1' })
    })

    writeTextFile('nexgen/restclient/data.yml', yml, { baseDir: BaseDirectory.Document }).then(() => {
      console.log('write successful')
    }).catch(console.error);
  }

  const openWorkspace = async () => {
    const selected = await open({ title: 'Select workpsace', directory: true, multiple: false });
    if (selected) {
      setWD(selected)
    }
  }

  useEffect(() => {
    if (!wd) return;
    console.log('wd', wd);
    readDir(wd, {  }).then(directories => {
      console.log(JSON.stringify(directories))
    }).catch(console.error)
  }, [wd])

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
              <button className="block data-[focus]:bg-blue-100" onClick={createWorkspace}>
                New Workspace
              </button>
            </MenuItem>
            <MenuItem>
              <button className="block data-[focus]:bg-blue-100" onClick={openWorkspace}>
                Open Workspace
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
