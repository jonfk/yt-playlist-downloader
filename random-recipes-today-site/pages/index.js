import Head from 'next/head'
import Image from 'next/image'
import styles from '../styles/Home.module.css'
import { promises as fs } from 'fs'
import path from 'path'

export default function Home({ videos }) {
  return (
    <div>
      <Head>
        <title>Random Recipes Today</title>
        <meta name="description" content="random recipes today" />
        <link rel="icon" href="/favicon.ico" />
      </Head>

      <main>
        <h1>
          Random Recipes Today
        </h1>
        <div>
          <ol>
            {videos.map((video) => (
              <li>
                {video.title}
              </li>
            ))}
          </ol>
        </div>
      </main>

      <footer className={styles.footer}>
        Powered by{' '}
        <span className={styles.logo}>
          {/* <Image src="/vercel.svg" alt="Vercel Logo" width={72} height={16} /> */}
        </span>
      </footer>
    </div>
  )
}

export async function getStaticProps(context) {
  const videosDirectory = path.join(process.cwd(), '/../videos');
  const filenames = await fs.readdir(videosDirectory);

  const videos = await Promise.all(filenames.map(async (filename) => {
    const filePath = path.join(videosDirectory, filename)
    const fileContents = await fs.readFile(filePath, 'utf8')
    // console.log(fileContents);

    return JSON.parse(fileContents);
  }));

  return {
    props: {
      videos: videos,
    }, // will be passed to the page component as props
  }
}
