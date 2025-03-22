# Studio-Whip
Open source AI content production suite for movies and visual novels. Create compelling stories with seamlessly integrated image, video, text, and audio models. Our goal is not to replace creative talent, but enhance it with workflows and tools that can rapidly spark and refine ideas.

Enjoy complete privacy, customizability, and controlled costs with local model inference.

### Major Planned Features:
- __P2P real time collaboration:__ Free and easy access to remote group projects without any subscriptions or servers.
- __Image 2 Image:__ With a powerful brush engine, 3D scene rendering, and refrence library.
- __Story driven interface:__ The quality of any show is ultimatly dependant on its ability to tell captivating stories. Thats why we prioritize creative writing with advanced LLM integrations to guide the production process.
- Node Editor
- Story Boarding
- XML Multimedia Sequencing
- Color Managed Workspace
- Color Grading (Davinci-like)

## Requirments (All platforms)
- Vulkan 1.3 or later
- Rustup (Latest Stable)
- Nvidia GPU with 16GB+ VRAM recomended


## Setup On Windows
### Windows Dependency Download Links
- Vulkan 1.3 or later : https://vulkan.lunarg.com/sdk/home#windows
- Rustup (Latest Stable) : https://www.rust-lang.org/tools/install

### Environment Variables
After instalation of all dependencys, you will need to add an environment variable to Path pointing to VulkanSDK GLSLC. 

1. Press ```Win+r``` and enter ```SystemPropertiesAdvanced```
2. Go to ```Environment Variables...```
3. Select ```Path``` then ```Edit```
4. Click ```New``` and add the value ```C:\VulkanSDK\<version>\Bin```
5. Press ```OK```
6. You can varify it worked using ```glslc --version``` in powershell.
7. Restart your IDE if it  was running while updating ```Path```

### Running PowerShell Scripts
There are severeal optional powershell utility scripts that can be used for development. You might encounter the below error or notice the scripts dont do anything when run

```
<scriptName>.ps1 cannot be loaded because running scripts is disabled on this 
system. For more information, see about_Execution_Policies at https:/go.microsoft.com/fwlink/?LinkID=135170.
    + CategoryInfo          : SecurityError: (:) [], ParentContainsErrorRecordException
    + FullyQualifiedErrorId : UnauthorizedAccess
```

To fix this
1. ```Win+r``` > ```powershell``` > ```Ctrl+Shift+Enter``` (to run as admin)
2. Run ```Set-ExecutionPolicy -Scope CurrentUser -ExecutionPolicy RemoteSigned```
3. You should now be able to run the *.ps1 scripts.