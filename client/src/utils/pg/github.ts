import { PgCommon } from "./common";
import { PgExplorer, TupleFiles } from "./explorer";
import { PgFramework } from "./framework";
import { GithubError } from "../../constants";

type GithubRepositoryData = {
  name: string;
  path: string;
  sha: string;
  size: number;
  url: string;
  html_url: string;
  git_url: string;
} & (
  | {
      type: "file";
      download_url: string;
    }
  | {
      type: "dir";
      download_url: null;
    }
);

type GithubRepositoryResponse = GithubRepositoryData[];

export class PgGithub {
  /**
   * Create a new workspace from the given GitHub URL.
   *
   * @param url GitHub URL
   */
  static async import(url: string) {
    // Get repository
    const { files, owner, repo, path } = await this._getRepository(url);

    // Check whether the repository already exists in user's workspaces
    const githubWorkspaceName = `github-${owner}/${repo}/${path}`;
    if (PgExplorer.allWorkspaceNames?.includes(githubWorkspaceName)) {
      // Switch to the existing workspace
      await PgExplorer.switchWorkspace(githubWorkspaceName);
    } else {
      // Create a new workspace
      const convertedFiles = await PgFramework.convertToPlaygroundLayout(files);
      await PgExplorer.newWorkspace(githubWorkspaceName, {
        files: convertedFiles,
        skipNameValidation: true,
      });
    }
  }

  /**
   * Get the files from the given repository and map them to `ExplorerFiles`.
   *
   * @param url GitHub URL
   * @returns explorer files
   */
  static async getExplorerFiles(url: string) {
    const { files } = await this._getRepository(url);
    const convertedFiles = await PgFramework.convertToPlaygroundLayout(files);
    return PgExplorer.convertToExplorerFiles(convertedFiles);
  }

  /**
   * Get Github repository data and map the files to `TupleFiles`.
   *
   * @param url Github link to the program's folder in the repository
   * @returns files, owner, repo, path
   */
  private static async _getRepository(url: string) {
    const { data, owner, repo, path } = await this._getRepositoryData(url);

    const files: TupleFiles = [];
    const recursivelyGetFiles = async (
      dirData: GithubRepositoryResponse,
      currentUrl: string
    ) => {
      // TODO: Filter `dirData` to only include the files we could need
      // Fetching all files one by one and just returning them without dealing
      // with any of the framework related checks is great here but it comes
      // with the cost of using excessive amounts of network requests to fetch
      // bigger repositories. This is especially a problem if the repository we
      // are fetching have unrelated files in their program workspace folder.
      for (const itemData of dirData) {
        if (itemData.type === "file") {
          const content = await PgCommon.fetchText(itemData.download_url!);
          files.push([itemData.path, content]);
        } else if (itemData.type === "dir") {
          const insideDirUrl = PgCommon.joinPaths([currentUrl, itemData.name]);
          const { data: insideDirData } = await this._getRepositoryData(
            insideDirUrl
          );
          await recursivelyGetFiles(insideDirData, insideDirUrl);
        }
      }
    };
    await recursivelyGetFiles(data, url);

    return { files, owner, repo, path };
  }

  /**
   * Get GitHub repository data.
   *
   * @param url GitHub link to the program's folder in the repository
   * @returns GitHub repository data, owner, repo, path
   */
  private static async _getRepositoryData(url: string) {
    // https://github.com/solana-labs/solana-program-library/tree/master/token/program
    const regex = new RegExp(
      /(https:\/\/)?(github\.com\/)([\w-]+)\/([\w-]+)(\/)?((tree|blob)\/([\w-.]+))?(\/)?([\w-/.]*)/
    );
    const res = regex.exec(url);
    if (!res) throw new Error(GithubError.INVALID_URL);

    const owner = res[3]; // solana-labs
    const repo = res[4]; // solana-program-library
    const ref = res[8]; // master
    const path = res[10]; // token/program

    // If it's a single file fetch request, Github returns an object instead of an array
    const data: GithubRepositoryResponse | GithubRepositoryData =
      await PgCommon.fetchJSON(
        `https://api.github.com/repos/${owner}/${repo}/contents/${path}?ref=${ref}`
      );

    return { data: PgCommon.toArray(data), owner, repo, path };
  }
}
