import {Composition, Still} from 'remotion';
import {Promo} from './Promo';
import {Cover} from './Cover';

export const Root = () => (
  <>
    <Composition id="Promo" component={Promo} durationInFrames={660} fps={30} width={1920} height={1080} />
    <Still id="Cover" component={Cover} width={1920} height={1200} />
  </>
);
