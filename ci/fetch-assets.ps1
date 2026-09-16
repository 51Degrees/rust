# The on-premise data files the tests read are Git LFS content inside the
# submodules of the C and C++ source repositories, so they come down in
# setup-environment.ps1 with the checkouts that need them rather than here.
Write-Output "Assets come with the native source checkouts"
