/* *********************************************************************
 * This Original Work is copyright of 51 Degrees Mobile Experts Limited.
 * Copyright 2026 51 Degrees Mobile Experts Limited, Davidson House,
 * Forbury Square, Reading, Berkshire, United Kingdom RG1 3EU.
 *
 * This Original Work is licensed under the European Union Public Licence
 * (EUPL) v.1.2 and is subject to its terms as set out below.
 *
 * If a copy of the EUPL was not distributed with this file, You can obtain
 * one at https://opensource.org/licenses/EUPL-1.2.
 *
 * The 'Compatible Licences' set out in the Appendix to the EUPL (as may be
 * amended by the European Commission) shall be deemed incompatible for
 * the purposes of the Work and the provisions of the compatibility
 * clause in Article 5 of the EUPL shall not apply.
 *
 * If using the Work as, or as part of, a network application, by
 * including the attribution notice(s) required under Article 5 of the EUPL
 * in the end user terms of the application under an appropriate heading,
 * such notice(s) shall fulfill the requirements of that article.
 * ********************************************************************* */

#include "file.h"
#include "fiftyone.h"

#include <inttypes.h>

#ifdef _MSC_VER
#include <windows.h>
#else
#include <stdio.h>
#include <unistd.h>
#endif

#ifdef __APPLE__
#include <libproc.h>
#include <sys/proc_info.h>
#endif

 /* Compare method passed to the iterate method. */
typedef bool(*fileCompare)(const char*, void*);

/* Match method called by the iterate method if the compare method returns true. */
typedef bool(*fileMatch)(const char*, void*);

/* State passed to the iterator methods. */
typedef struct fileIteratorState_t {
	const char *masterFileName; /* Path to the master file */
	const char *baseName; /* Base name of the master file (i.e. without
							the path and extension) */
	size_t baseNameLength; /* Length of baseName */
	const char *destination; /* Memory to write the matching file path to */
	FileOffset bytesToCompare; /* Number of bytes to compare from the start of the
						 file */
} fileIteratorState;

#ifdef _MSC_VER
#pragma warning (disable:4100)  
#endif

static void* createFileHandle(Pool *pool, void *state, Exception *exception) {
	FILE *fileHandle;
	StatusCode status = FileOpen(
		(const char*)state,
		&fileHandle);
	if (status != SUCCESS) {
		EXCEPTION_SET(status)
		fileHandle = NULL;
	}
	return (void*)fileHandle;
}

static void freeFileHandle(Pool *pool, void *fileHandle) {
	fclose((FILE*)fileHandle);
}

#ifdef _MSC_VER
#pragma warning (default:4100)  
#endif

int fiftyoneDegreesFileSeek(
   FILE * const stream,
   const FileOffset offset,
   const int origin) {

	return
#ifdef FIFTYONE_DEGREES_LARGE_DATA_FILE_SUPPORT
#	ifdef _MSC_VER
	_fseeki64
#	else
	fseeko
#	endif
#else
	fseek
#endif
	(stream, offset, origin);
}

FileOffset fiftyoneDegreesFileTell(FILE * const stream) {

	return
#ifdef FIFTYONE_DEGREES_LARGE_DATA_FILE_SUPPORT
#	ifdef _MSC_VER
	_ftelli64
#	else
	ftello
#	endif
#else
	ftell
#endif
	(stream);
}


static FileOffset fileGetSize(FILE *file) {
	if (FileSeek(file, 0L, SEEK_END) == 0) {
		return FileTell(file);
	}
	return -1;
}

static FileOffset setLength(FilePool *reader, Exception *exception) {
	FileHandle *handle;
	reader->length = 0;
	handle = FileHandleGet(reader, exception);
	if (handle != NULL && EXCEPTION_OKAY) {
		reader->length = fileGetSize(handle->file);
		FileHandleRelease(handle);
	}
	return reader->length;
}

static StatusCode fileOpen(
	const char* fileName,
	FILE** handle,
	const char *mode) {
	// Open the file and hold on to the data.ptr.
#ifndef _MSC_FULL_VER
	unsigned int originalMask = 0;
	if (strcmp(mode, "wb") == 0) {
		originalMask = umask(0);
	}

	*handle = fopen(fileName, mode);

	if (strcmp(mode, "wb") == 0) {
		umask(originalMask);
	}
	
	if (*handle == NULL) {
		if (strcmp(mode, "rb") == 0) {
			return FILE_NOT_FOUND;
		}
		else if (strcmp(mode, "wb") == 0) {
			return FILE_WRITE_ERROR;
		}
		else {
			return FILE_FAILURE;
		}
	}
#else
	/* If using Microsoft use the fopen_s method to avoid warning */
	errno_t error = fopen_s(handle, fileName, mode);
	if (error != 0) {
		switch (error) {
		case ENFILE:
		case EMFILE:
			return TOO_MANY_OPEN_FILES;
		case EACCES:
		case EROFS:
			return FILE_PERMISSION_DENIED;
		case EEXIST:
			return FILE_EXISTS_ERROR;
		case ENOENT:
		default:
			return FILE_NOT_FOUND;
		}
	}
#endif
	return SUCCESS;
}

static StatusCode fileCopy(FILE *source, FILE *destination) {
	unsigned char buffer[65536]; // Increased from 8KB to 64KB for better performance, especially on Windows
	size_t lengthRead, lengthWritten = 0;
	if (FileSeek(source, 0L, SEEK_END) == 0) {
		FileSeek(source, 0L, SEEK_SET);
		lengthRead = fread(buffer, 1, sizeof(buffer), source);
		while (lengthRead > 0) {
			lengthWritten = fwrite(buffer, 1, lengthRead, destination);
			if (lengthWritten != lengthRead) {
				return FILE_COPY_ERROR;
			}
			lengthRead = fread(buffer, 1, sizeof(buffer), source);
		}
	}
	return SUCCESS;
}

/**
 * Attempts to open a file to determine whether or not it exists.
 * @param fileName name of the file to check
 * @return true if the file exists
 */
static bool fileExists(const char *fileName) {
	FILE *file;
	if (fileOpen(fileName, &file, "rb") == SUCCESS) {
		fclose(file);
		return true;
	}
	return false;
}

static long getRandSeed() {
	struct timespec ts;
#ifdef _MSC_VER
	if (timespec_get(&ts, TIME_UTC) != 0) {
		return ts.tv_nsec;
	}
#else
	if (clock_gettime(CLOCK_REALTIME, &ts) == 0) {
		return ts.tv_nsec;
	}
#endif
	else {
		return -1;
	}
}

/**
 * Generates a random string consisting of random ASCII characters. The
 * random name is written to the destination parameter, and the number of
 * characters written are returned.
 * @param length the number of characters including null terminator
 * @param destination allocated memory where the string will be written
 * @return number of characters written. Negative if error occurs.
 */
static int getRandomString(
	size_t length,
	char *destination) {
	size_t i;

	// Seed the random function. If this is not done, then the same random
	// names will be generated each time an executable is run.
	long seed = getRandSeed();
	if (seed == -1) {
		return -1;
	}

	srand((unsigned int)seed);

	// Generate a random string using ASCII characters in the range [65,90)
	// i.e. uppercase alphabet.
	for (i = 0; i < length - 1; i++) {
		destination[i] = (char)(rand() % 26 + 65);
	}
	destination[length - 1] = '\0';

	return (int)length;
}

/**
 * Generates a unique file name consisting of random ASCII characters. The
 * unique name is written to the destination parameter, and the number of
 * characters written are returned. This is used by the old CreateTempFile
 * @param length the number of characters including null terminator
 * @param destination allocated memory where the string will be written
 * @param nameStart the character position to in destination to start writing
 * the file name
 * @return number of characters written
 */
static int getUniqueLegacyFileName(
	size_t length,
	char* destination,
	size_t nameStart) {
	int charAdded = 0;
	do {
		charAdded = getRandomString(length, destination + nameStart);
	} while (fileExists(destination) && (charAdded >= 0 && charAdded <= (int)length));
	return charAdded;
}

/**
 * Compare two files, first by their sizes, then the contents.
 * @param fileName first file to compare
 * @param otherFileName second file to compare
 * @param bytesToCompare number of from the start of the file to compare for
 * equality with the master file, or -1 to compare the whole file
 * @return true if files are identical
 */
static bool compareFiles(
	const char *fileName,
	const char *otherFileName,
	FileOffset bytesToCompare) {
	StatusCode status;
	FILE *file, *otherFile;
	FileOffset size;
	byte buffer[1024], otherBuffer[1024];

	// Compare the sizes.
	size = FileGetSize(fileName);
	if (size > 0 && size != FileGetSize(otherFileName)) {
		return false;
	}
	// Open both files.
	status = FileOpen(fileName, &file);
	if (status != SUCCESS) {
		return false;
	}
	status = FileOpen(otherFileName, &otherFile);
	if (status != SUCCESS) {
		fclose(file);
		return false;
	}

	while (FileTell(file) < size &&
		(bytesToCompare < 0 || FileTell(file) < bytesToCompare)) {
		size_t read = (
			((bytesToCompare > 0)
				&& (bytesToCompare <= (FileOffset)sizeof(buffer)))
			? (size_t)bytesToCompare
			: sizeof(buffer));

		if ((uint64_t)(size - FileTell(file)) < (uint64_t)read) {
			read = (size_t)(size - FileTell(file));
		}

		if (fread(buffer, read, 1, file) != 1) {
			fclose(file);
			fclose(otherFile);
			return false;
		}
		if (fread(otherBuffer, read, 1, otherFile) != 1) {
			fclose(file);
			fclose(otherFile);
			return false;
		}
		if (memcmp(buffer, otherBuffer, read) != 0) {
			fclose(file);
			fclose(otherFile);
			return false;
		}
	}
	fclose(file);
	fclose(otherFile);
	return true;
}

/**
 * Gets the file name from the full path.
 * @param path path to get the file name from
 * @return the file name in the path
 */
static const char* getNameFromPath(const char *path) {
	char *last = (char*)path,
		*current = (char*)path;
	while (*current != '\0') {
		if (*current == '/' || *current == '\\') {
			last = current + 1;
		}
		current++;
	}
	return last;
}

/**
 * Checks whether the names of two files are the same. The paths to the files
 * are not taken into account, as this would return false when comparing a
 * relative path to an absolute path.
 * @param path first path to compare
 * @param otherPath second path to compare
 * @return true if both file paths have the same file name
 */
static bool fileNamesMatch(const char *path, const char *otherPath) {
	return strcmp(
		getNameFromPath(path),
		getNameFromPath(otherPath)) == 0;
}

/**
 * Iterates over all files in the path provided. If a matching file is
 * identified by the compare method, then the match method is called and true
 * returned. No more iterations are performed after the first file is found.
 * @param path directory to iterate over
 * @param state pointer to a structure to pass to the compare and match methods
 * @param compare method called to check if the file is a match
 * @param match method called if a matching file is found
 * @return true if a matching file was found in the directory
 */
static bool iterateFiles(
	const char *path,
	void *state,
	fileCompare compare,
	fileMatch match) {
	char tempPath[FIFTYONE_DEGREES_FILE_MAX_PATH];
	size_t nameStart;
	// Append a slash to the path if one is not there already.
	strcpy(tempPath, path);
	nameStart = strlen(path);
	if (nameStart != 0 &&
		tempPath[nameStart - 1] != '/' &&
		tempPath[nameStart - 1] != '\\') {
		tempPath[nameStart++] = '/';
	}

#ifdef _MSC_VER
	wchar_t wString[FIFTYONE_DEGREES_FILE_MAX_PATH];
	WIN32_FIND_DATA file;
	HANDLE searchHandle;
	if (nameStart + 1 > FIFTYONE_DEGREES_FILE_MAX_PATH) {
		return false;
	}
	// Build the search pattern, e.g. /some/path/*
	tempPath[nameStart] = '*';
	tempPath[nameStart + 1] = '\0';
	// Convert the path to a wide char string.
	if (MultiByteToWideChar(
		CP_ACP,
		0,
		tempPath,
		-1,
		wString,
		FIFTYONE_DEGREES_FILE_MAX_PATH) == 0) {
		return false;
	}
	// Iterate over all files.
	if ((searchHandle = FindFirstFile(wString, &file))
		!= INVALID_HANDLE_VALUE) {
		do {
			// Append the name to the path.
			if (WideCharToMultiByte(
				CP_ACP,
				0,
				file.cFileName,
				-1,
				&tempPath[nameStart],
				(int)(sizeof(char) *
					(FIFTYONE_DEGREES_FILE_MAX_PATH - nameStart)),
				NULL,
				NULL) == 0) {
				FindClose(searchHandle);
				return false;
			}
			// Call match and return if the file is a match.
			if (compare(tempPath, state)) {
				if (match(tempPath, state) == true) {
					FindClose(searchHandle);
					return true;
				}
			}
		} while (FindNextFile(searchHandle, &file) != 0);
		FindClose(searchHandle);
	}
#else
	DIR *dir;
	struct dirent *ent;
	dir = opendir(path);
	if (dir != NULL) {
		// Iterate over all files.
		while ((ent = readdir(dir)) != NULL) {
			// Append the name to the path.
			strcpy(tempPath + nameStart, ent->d_name);
			// Call match and return if the file is a match.
			if (compare(tempPath, state)) {
				if (match(tempPath, state) == true) {
					closedir(dir);
					return true;
				}
			}
		}
		closedir(dir);
	}
#endif
	return false;
}

/**
 * Compares the file, and the master file who's name is stored in the state.
 * If the contents of the files are the same (up to the bytesToCompare limit)
 * and the file names are not the same, then true is returned.
 * @param fileName name of the file to compare to the master file
 * @param state pointer to the file iterator state with the search parameters
 * @return true if the files are equal, but different files
 */
static bool iteratorFileCompare(const char *fileName, void *state) {
	fileIteratorState *fileState = (fileIteratorState*)state;
	if (strncmp(fileState->baseName, getNameFromPath(fileName), fileState->baseNameLength) == 0 &&
		compareFiles(
		fileState->masterFileName,
		fileName,
		fileState->bytesToCompare) == true &&
		fileNamesMatch(fileName, fileState->masterFileName) == false) {
		return true;
	}
	return false;
}

/**
 * Copies the name of the file to the destination memory pointed to by the
 * state structure.
 * @param fileName path to the matching file
 * @param state pointer to the file iterator state with the destination pointer
 * @return true to indicate that the search should end
 */
static bool iteratorFileMatch(const char *fileName, void *state) {
	fileIteratorState *fileState = (fileIteratorState*)state;
	strcpy((char*)fileState->destination, fileName);
	return true;
}

#ifdef _MSC_VER
// For MSC version, the parameter is not required
#pragma warning (disable: 4100)
#endif
#ifdef __linux__
/**
 * Returns true if the file is in use. Note that this is only functional on
 * Linux systems. Windows does not need this for the usage in this file.
 * The Apple implementation is currently unstable (so is not used). Also note
 * that the file MUST exist. If it does not, then this method will result in
 * undefined behaviour.
 * @param pathName path to the file to check
 * @return true if the file is in use
 */
static bool isFileInUse(const char *pathName) {
	DIR *procDir;
	struct dirent *ent1, *ent2;
	char fdPath[FILE_MAX_PATH];
	char linkFile[FILE_MAX_PATH];
	char linkPath[FILE_MAX_PATH];
	procDir = opendir("/proc");
	if (procDir != NULL) {
		// Iterate over all directories in /proc
		while ((ent1 = readdir(procDir)) != NULL) {
			// Get the path to the file descriptor directory for a PID
			Snprintf(fdPath, FILE_MAX_PATH, "/proc/%s/fd", ent1->d_name);
			DIR *fdDir = opendir(fdPath);
			if (fdDir != NULL) {
				while ((ent2 = readdir(fdDir)) != NULL) {
					// Check that the file is not '.' or '..'
					if (strcmp(ent2->d_name, ".") != 0 &&
						strcmp(ent2->d_name, "..") != 0) {
						// Get the path which the symlink is pointing to
						if (Snprintf(
							linkFile,
							FILE_MAX_PATH,
							"%s/%s",
							fdPath,
							ent2->d_name) >= 0) {
							ssize_t written =
								readlink(linkFile, linkPath, FILE_MAX_PATH);
							if (written >= 0) {
								linkPath[written] = '\0';
								size_t linkPathLen = strlen(linkPath);
								size_t pathNameLen = strlen(pathName);
								if (pathNameLen <= linkPathLen &&
									strncmp(linkPath + linkPathLen - pathNameLen,
										pathName,
										pathNameLen) == 0) {
									closedir(fdDir);
									closedir(procDir);
									return true;
								}
							}
						}
					}
				}
			}
			closedir(fdDir);
		}
		closedir(procDir);
	}
	else {
		// This method cannot be used to determine whether the file is in use.
		// So to be safe, lets report true so it is not deleted.
		return true;
	}
	return false;
}
#endif

#ifdef _MSC_VER
#pragma warning (default: 4100)
#endif

/**
 * Deletes the file is it is not in use. The first byte of state->destination
 * is used as a counter to indicate how many files were successfully deleted.
 * @param fileName path to matching file
 * @param state pointer to the file iterator state with the destination pointer
 * @return false to indicate that the search should continue
 */
static bool iteratorFileDelete(const char *fileName, void *state) {
	fileIteratorState *fileState = (fileIteratorState*)state;
#ifdef __linux__
	/*
	  On non Windows platforms, the file locking is advisory. Therefore,
	  an explicit check is required. Currently no stable solution has been
	  implemented for MacOS platform so only perform this for Linux.
	  TODO: Implement a 'isFileInUse' solution for MacOS.
	*/
	if (isFileInUse(fileName) == false) {
#endif
		if (fiftyoneDegreesFileDelete(fileName) == SUCCESS) {
			((byte*)fileState->destination)[0]++;
		}
#ifdef __linux__
	}
#endif
	return false;
}

fiftyoneDegreesStatusCode fiftyoneDegreesFileOpen(
	const char* fileName,
	FILE** handle) {
	return fileOpen(fileName, handle, "rb");
}

fiftyoneDegreesStatusCode fiftyoneDegreesFileWrite(
	const char* fileName,
	const void *data,
	const size_t length) {
	FILE *file;
	StatusCode status = fileOpen(fileName, &file, "wb");
	if (status == SUCCESS) {
		if (fwrite(data, length, 1, file) != 1) {
			status = FILE_FAILURE;
		}
		fclose(file);
	}
	return status;
}

fiftyoneDegreesStatusCode fiftyoneDegreesFileCreateDirectory(
	const char *pathName) {
#ifdef _MSC_VER
	int error = _mkdir(pathName);
#else
#ifdef __MINGW32__
	int error = mkdir(pathName);
#else
	int error = mkdir(pathName, S_IRUSR | S_IWUSR | S_IXUSR);
#endif
#endif
	if (error != 0) {
		switch (errno) {
		case 0:
		case EACCES:
		case EROFS:
			return FILE_PERMISSION_DENIED;
		case EEXIST:
			return FILE_EXISTS_ERROR;
		case ENOENT:
		default:
			return FILE_NOT_FOUND;
		}
	}
	return SUCCESS;
}

static fiftyoneDegreesStatusCode getBasenameWithoutExtension(
	const char* path,
	char* dest,
	size_t length) {
	StatusCode status = NOT_SET;
	const char* pos = getNameFromPath(path);
	char* dot = strrchr(path, '.');
	size_t end = strlen(pos);
	if (dot != NULL) {
		end = dot - pos;
	}

	if (end + 1 > length) {
		status = INSUFFICIENT_MEMORY;
	}
	else {
		strncpy(dest, pos, end);
		dest[end] = '\0';
		status = SUCCESS;
	}
	return status;
}

int fiftyoneDegreesFileDeleteUnusedTempFiles(
	const char *masterFileName,
	const char **paths,
	int count,
	FileOffset bytesToCompare) {
	int i;
	byte deleted = 0;
	fileIteratorState state;
	state.masterFileName = masterFileName;
	// The iteratorFileDelete method will use the first byte of
	// state.destination to keep track of the number of files deleted. This is
	// a slight misuse of the structure, but we'll allow it as the structure
	// is internal only.
	state.destination = (const char*)&deleted;
	state.bytesToCompare = bytesToCompare;

	char basename[FIFTYONE_DEGREES_FILE_MAX_PATH];
	StatusCode status = getBasenameWithoutExtension(
		masterFileName, basename, FIFTYONE_DEGREES_FILE_MAX_PATH);
	if (status != SUCCESS) {
		return 0;
	}
	state.baseName = basename;
	state.baseNameLength = strlen(basename);

	if (paths == NULL || count == 0) {
		// Look in the working directory.
		iterateFiles("", &state, iteratorFileCompare, iteratorFileDelete);
	}
	else {
		// Look in the directories provided.
		for (i = 0; i < count; i++) {
			iterateFiles(
				paths[0],
				&state,
				iteratorFileCompare,
				iteratorFileDelete);
		}
	}
	return (int)deleted;
}

bool fiftyoneDegreesFileGetExistingTempFile(
	const char *masterFileName,
	const char **paths,
	int count,
	FileOffset bytesToCompare,
	const char *destination) {
	int i;
	fileIteratorState state;
	char basename[FIFTYONE_DEGREES_FILE_MAX_PATH];

	state.masterFileName = masterFileName;
	state.destination = destination;
	state.bytesToCompare = bytesToCompare;
	StatusCode status = getBasenameWithoutExtension(
		masterFileName, basename, FIFTYONE_DEGREES_FILE_MAX_PATH);
	if (status != SUCCESS) {
		return 0;
	}
	state.baseName = basename;
	state.baseNameLength = strlen(basename);

	if (paths == NULL || count == 0) {
		// Look in the working directory.
		return iterateFiles("", &state, iteratorFileCompare, iteratorFileMatch);
	}
	else {
		// Look in the directories provided.
		for (i = 0; i < count; i++) {
			if (iterateFiles(
				paths[0],
				&state,
				iteratorFileCompare,
				iteratorFileMatch) == true) {
				return true;
			}
		}
	}
	return false;
}

fiftyoneDegreesStatusCode fiftyoneDegreesFileAddTempFileName(
	const char* masterFileName,
	char* destination,
	size_t nameStart,
	size_t length) {
	char uniqueString[TEMP_UNIQUE_STRING_LENGTH + 1];
	char basename[FIFTYONE_DEGREES_FILE_MAX_PATH];
	uint64_t processId = ProcessGetId();
	StatusCode status = getBasenameWithoutExtension(
		masterFileName, basename, FIFTYONE_DEGREES_FILE_MAX_PATH);
	if (status != SUCCESS) {
		return status;
	}

	do {
		if (getRandomString(TEMP_UNIQUE_STRING_LENGTH + 1, uniqueString) < 0) {
			return TEMP_FILE_ERROR;
		}

		int charsAdded = Snprintf(
			destination + nameStart,
			length,
			"%s-%" PRId64 "-%s",
			basename,
			processId,
			uniqueString);
	
		if (charsAdded < 0) {
			status = ENCODING_ERROR;
		}
		else if ((size_t)charsAdded > length) {
			status = INSUFFICIENT_MEMORY;
		}
		else {
			status = SUCCESS;
		}
	
		// Discard any changes to the destination if error occurred
		if (status != NOT_SET && status != SUCCESS) {
			memset(destination + nameStart, 0, length);
		}
	} while (status == SUCCESS && fileExists(destination));
	return status;
}

const char* fiftyoneDegreesFileGetBaseName(const char *path) {
	char* lastSlash = NULL;
	if ((lastSlash = strrchr(path, '/')) == NULL) {
		lastSlash = strrchr(path, '\\');
		if (lastSlash != NULL && strlen(lastSlash) != 1) {
			return lastSlash + 1;
		}
	}
	return NULL;
}

static fiftyoneDegreesStatusCode createTempFileWithoutPaths(
	const char* masterFile,
	char* destination,
	size_t length) {
	const char* masterFileName = getNameFromPath(masterFile);
	StatusCode status = fiftyoneDegreesFileAddTempFileName(
		masterFileName, destination, 0, length);
	if (status != SUCCESS) {
		return status;
	}

	status = FileCopy(masterFile, destination);
	if (status != SUCCESS && length > 0) {
		memset(destination, 0, length);
	}
	return status;
}

static fiftyoneDegreesStatusCode createTempFileWithPaths(
	const char* masterFile,
	const char** paths,
	int count,
	char* destination,
	size_t length) {
	size_t nameStart;
	StatusCode status = NOT_SET;
	const char* masterFileName = getNameFromPath(masterFile);

	for (int i = 0; i < count; i++) {
		int charAdded = 0;
		nameStart = strlen(paths[i]);
		if (nameStart != 0 &&
			paths[i][nameStart - 1] != '/' &&
			paths[i][nameStart - 1] != '\\') {
			charAdded = Snprintf(
				destination, length, "%s/", paths[i]);
			nameStart++;
		}
		else {
			charAdded = Snprintf(
				destination, length, "%s", paths[i]);
		}

		if (charAdded < 0) {
			status = ENCODING_ERROR;
		}
		else if ((size_t)charAdded > length) {
			status = INSUFFICIENT_MEMORY;
		}
		else {
			status = fiftyoneDegreesFileAddTempFileName(
				masterFileName, destination, nameStart, length - nameStart - 1);
			if (status == SUCCESS) {
				// Create the temp file
				status = FileCopy(masterFile, destination);
			}
		}

		// Reset the destination
		if (status != SUCCESS && length > 0) {
			memset(destination, 0, length);
		}
	}
	return status;
}

fiftyoneDegreesStatusCode fiftyoneDegreesFileNewTempFile(
	const char* masterFile,
	const char** paths,
	int count,
	char* destination,
	size_t length) {
	StatusCode status = NOT_SET;

	if (paths == NULL || count == 0) {
		status = createTempFileWithoutPaths(
			masterFile, destination, length);
	}
	else if (paths != NULL) {
		status = createTempFileWithPaths(
			masterFile, paths, count, destination, length);
	}
	assert(status != NOT_SET);
	return status;
}

fiftyoneDegreesStatusCode fiftyoneDegreesFileCreateTempFile(
	const char *masterFile,
	const char **paths,
	int count,
	const char *destination) {
	int i;
	size_t nameStart;
	StatusCode status = NOT_SET;
	char fileName[TEMP_UNIQUE_STRING_LENGTH];
	char tempPath[FIFTYONE_DEGREES_FILE_MAX_PATH];

	if (paths == NULL || count == 0) {
		getUniqueLegacyFileName(TEMP_UNIQUE_STRING_LENGTH, fileName, 0);
		status = FileCopy(masterFile, fileName);
		if (status == SUCCESS) {
			strcpy((char*)destination, fileName);
			return status;
		}
	}
	else if (paths != NULL) {
		for (i = 0; i < count; i++) {
			strcpy(tempPath, paths[i]);
			nameStart = strlen(paths[i]);
			if (nameStart != 0 &&
				tempPath[nameStart - 1] != '/' &&
				tempPath[nameStart - 1] != '\\') {
				tempPath[nameStart++] = '/';
			}
			if (nameStart + TEMP_UNIQUE_STRING_LENGTH < FIFTYONE_DEGREES_FILE_MAX_PATH) {
				getUniqueLegacyFileName(TEMP_UNIQUE_STRING_LENGTH, tempPath, nameStart);
				status = FileCopy(masterFile, tempPath);
				if (status == SUCCESS) {
					strcpy((char*)destination, tempPath);
					return status;
				}
			}
			else {
				status = FILE_PATH_TOO_LONG;
			}
		}
	}
	assert(status != NOT_SET);
	return status;
}

fiftyoneDegreesStatusCode fiftyoneDegreesFileGetPath(
	const char *dataFolderName,
	const char *fileName,
	char *destination,
	size_t size) {
	size_t i;
	FILE *handle;
	int charsWritten;
	char testPath[FIFTYONE_DEGREES_FILE_MAX_PATH];
	if (GET_CURRENT_DIR(
		testPath, 
		FIFTYONE_DEGREES_FILE_MAX_PATH) == testPath) {
		for (i = strlen(testPath); i > 0; i--) {
			if (testPath[i] == '\\' || testPath[i] == '/' ||
				testPath[i] == '\0') {
				charsWritten = Snprintf(
					testPath + i,
					sizeof(testPath) - (i * sizeof(char)),
					"/%s/%s",
					dataFolderName,
					fileName);
				if (charsWritten < 0) {
					return ENCODING_ERROR;
				}
				if ((size_t)charsWritten >=
					(sizeof(testPath) / sizeof(char)) - i) {
					return INSUFFICIENT_MEMORY;
				}
				if (fileOpen(testPath, &handle, "rb") ==
					SUCCESS) {
					fclose(handle);
					charsWritten = Snprintf(destination, size, "%s", testPath);
					if (charsWritten < 0) {
						return ENCODING_ERROR;
					}
					if ((size_t)charsWritten >= size / sizeof(char)) {
						return INSUFFICIENT_MEMORY;
					}
					return SUCCESS;
				}
			}
		}
	}
	return FILE_NOT_FOUND;
}

fiftyoneDegreesStatusCode fiftyoneDegreesFilePoolInit(
	fiftyoneDegreesFilePool *filePool,
	const char *fileName,
	uint16_t concurrency,
	fiftyoneDegreesException *exception) {
	StatusCode status = SUCCESS;
	if (concurrency <= 0) {
		return INVALID_COLLECTION_CONFIG;
	}
	if (PoolInit(
			&filePool->pool,
			concurrency,
			(void*)fileName,
			createFileHandle,
			freeFileHandle,
			exception) != NULL &&
		EXCEPTION_OKAY) {
		if (setLength(filePool, exception) == 0) {
			status = FILE_FAILURE;
		}
	}
	else if (EXCEPTION_FAILED) {
#ifndef FIFTYONE_DEGREES_EXCEPTIONS_DISABLED
		status = exception->status;
#endif
	}
	else {
		status = INSUFFICIENT_MEMORY;
	}
	return status;
}

fiftyoneDegreesFileHandle* fiftyoneDegreesFileHandleGet(
	fiftyoneDegreesFilePool *filePool,
	fiftyoneDegreesException *exception) {
	return (FileHandle*)PoolItemGet(&filePool->pool, exception);
}

void fiftyoneDegreesFileHandleRelease(fiftyoneDegreesFileHandle* handle) {
	PoolItemRelease(&handle->item);
}

void fiftyoneDegreesFilePoolRelease(fiftyoneDegreesFilePool* filePool) {
	PoolFree(&filePool->pool);
}

fiftyoneDegreesStatusCode fiftyoneDegreesFileDelete(const char *fileName) {
	if (remove(fileName) != 0) {
		switch (errno) {
		case EBUSY:
			return FILE_BUSY;
		case ENOENT:
			return FILE_NOT_FOUND;
		default:
			return FILE_FAILURE;
		}
	}
	return SUCCESS;
}

fiftyoneDegreesStatusCode fiftyoneDegreesFileCopy(
	const char *source,
	const char *destination) {
	FILE *sourceFile = NULL;
	FILE *destinationFile = NULL;
	StatusCode status = FileOpen(
		source,
		&sourceFile);
	if (status == SUCCESS) {
		status = fileOpen(destination, &destinationFile, "wb");
		if (status != SUCCESS) {
			fclose(sourceFile);
			return status;
		}
		status = fileCopy(sourceFile, destinationFile);
		fclose(destinationFile);
		// Delete the destination file if the status code is not success.
		if (status != SUCCESS) {
			FileDelete(destination);
		}
		fclose(sourceFile);
	}
	return status;
}

FileOffset fiftyoneDegreesFileGetSize(const char *source) {
	FILE *sourceFile;
	FileOffset size = -1;
	if (FileOpen(source, &sourceFile) == SUCCESS) {
		size = fileGetSize(sourceFile);
		fclose(sourceFile);
	}
	return size;
}

fiftyoneDegreesStatusCode fiftyoneDegreesFileReadToByteArray(
	const char *source,
	fiftyoneDegreesMemoryReader *reader) {
	FILE *sourceFile;
	StatusCode status = FileOpen(source, &sourceFile);
	if (status == SUCCESS) {
		// Get the size of the file and allocate sufficient memory.
		reader->length = fileGetSize(sourceFile);
		if (reader->length < 0) {
			status = FILE_FAILURE;
		} else if ((uint64_t)reader->length > (uint64_t)SIZE_MAX) {
			status = FILE_TOO_LARGE;
		} else {
			size_t const fileSize = (size_t)(reader->length * sizeof(char));
			reader->current = reader->startByte = (byte*)Malloc(fileSize);
			if (reader->current != NULL) {
				if (FileSeek(sourceFile, 0L, SEEK_SET) != 0 ||
					fread(reader->current, fileSize, 1, sourceFile) != 1) {
					// The file could not be loaded into memory. Release the
					// memory allocated earlier and set the status to file
					// failure.
					free(reader->current);
					reader->startByte = NULL;
					reader->current = NULL;
					reader->length = 0;
					status = FILE_FAILURE;
				}
				else {
					// Set the last byte to validate that the entire data structure
					// has been read.
					reader->lastByte = reader->current + reader->length;
				}
			}
			else {
				// Sufficient memory could not be allocated.
				reader->current = NULL;
				reader->length = 0;
				status = INSUFFICIENT_MEMORY;
			}
		}
		fclose(sourceFile);
	}
	return status;
}

void fiftyoneDegreesFilePoolReset(fiftyoneDegreesFilePool *filePool) {
	PoolReset(&filePool->pool);
	filePool->length = 0;
}

const char* fiftyoneDegreesFileGetFileName(const char *filePath) {
	char *c = (char*)filePath + strlen(filePath);
	while (*c != '\\' && *c != '/' && c != filePath) {
		c--;
	}
	return c + 1;
}
